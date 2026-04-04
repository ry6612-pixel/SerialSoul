package com.novaclaw.bleconfig

import android.annotation.SuppressLint
import android.bluetooth.*
import android.bluetooth.le.*
import android.content.Context
import android.os.ParcelUuid
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import java.util.UUID
import java.util.concurrent.ConcurrentLinkedQueue

/**
 * NovaClaw BLE Manager — connects to ESP32 via Nordic UART Service (NUS).
 *
 * Protocol:
 *   AUTH <pin>         → OK / FAIL
 *   GET <key>          → OK <value>
 *   SET <key> <value>  → OK
 *   LIST               → comma-separated keys
 *   SAVE               → OK RESTART (device reboots)
 *   PING               → PONG NovaClaw vX.Y.Z
 */
class BleManager(private val context: Context) {

    companion object {
        val NUS_SERVICE_UUID: UUID  = UUID.fromString("6e400001-b5a3-f393-e0a9-e50e24dcca9e")
        val NUS_RX_UUID: UUID      = UUID.fromString("6e400002-b5a3-f393-e0a9-e50e24dcca9e") // write
        val NUS_TX_UUID: UUID      = UUID.fromString("6e400003-b5a3-f393-e0a9-e50e24dcca9e") // notify
        private const val CCCD_UUID_STR = "00002902-0000-1000-8000-00805f9b34fb"
    }

    sealed class ConnectionState {
        data object Disconnected : ConnectionState()
        data object Scanning : ConnectionState()
        data object Connecting : ConnectionState()
        data object Connected : ConnectionState()
        data object Authenticated : ConnectionState()
    }

    data class ScannedDevice(val name: String, val address: String, val rssi: Int, val device: BluetoothDevice)

    private val _state = MutableStateFlow<ConnectionState>(ConnectionState.Disconnected)
    val state: StateFlow<ConnectionState> = _state

    private val _devices = MutableStateFlow<List<ScannedDevice>>(emptyList())
    val devices: StateFlow<List<ScannedDevice>> = _devices

    private val _response = MutableStateFlow("")
    val response: StateFlow<String> = _response

    private val _log = MutableStateFlow<List<String>>(emptyList())
    val log: StateFlow<List<String>> = _log

    private var bluetoothGatt: BluetoothGatt? = null
    private var rxChar: BluetoothGattCharacteristic? = null
    private var scanner: BluetoothLeScanner? = null
    private var responseCallback: CompletableDeferred<String>? = null
    private val pendingResponses = ConcurrentLinkedQueue<CompletableDeferred<String>>()

    private fun addLog(msg: String) {
        _log.value = (_log.value + msg).takeLast(100)
    }

    // ── Scan ──

    private val scanCallback = object : ScanCallback() {
        @SuppressLint("MissingPermission")
        override fun onScanResult(callbackType: Int, result: ScanResult) {
            val name = result.device.name ?: return
            if (!name.contains("NovaClaw", ignoreCase = true)) return
            val existing = _devices.value.toMutableList()
            val idx = existing.indexOfFirst { it.address == result.device.address }
            val item = ScannedDevice(name, result.device.address, result.rssi, result.device)
            if (idx >= 0) existing[idx] = item else existing.add(item)
            _devices.value = existing
        }
    }

    @SuppressLint("MissingPermission")
    fun startScan() {
        _devices.value = emptyList()
        _state.value = ConnectionState.Scanning
        val adapter = BluetoothAdapter.getDefaultAdapter() ?: return
        scanner = adapter.bluetoothLeScanner
        val filter = ScanFilter.Builder()
            .setServiceUuid(ParcelUuid(NUS_SERVICE_UUID))
            .build()
        val settings = ScanSettings.Builder()
            .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
            .build()
        scanner?.startScan(listOf(filter), settings, scanCallback)
        addLog("掃描中...")

        // Also scan without filter (some devices don't advertise service UUID)
        CoroutineScope(Dispatchers.Main).launch {
            delay(800)
            scanner?.stopScan(scanCallback)
            val settingsBalanced = ScanSettings.Builder()
                .setScanMode(ScanSettings.SCAN_MODE_BALANCED)
                .build()
            scanner?.startScan(null, settingsBalanced, scanCallback)
        }
    }

    @SuppressLint("MissingPermission")
    fun stopScan() {
        scanner?.stopScan(scanCallback)
        if (_state.value == ConnectionState.Scanning) {
            _state.value = ConnectionState.Disconnected
        }
    }

    // ── Connect ──

    @SuppressLint("MissingPermission")
    fun connect(device: BluetoothDevice) {
        stopScan()
        _state.value = ConnectionState.Connecting
        addLog("連線中: ${device.address}")
        bluetoothGatt = device.connectGatt(context, false, gattCallback, BluetoothDevice.TRANSPORT_LE)
    }

    @SuppressLint("MissingPermission")
    fun disconnect() {
        bluetoothGatt?.disconnect()
        bluetoothGatt?.close()
        bluetoothGatt = null
        rxChar = null
        _state.value = ConnectionState.Disconnected
        addLog("已斷線")
    }

    private val gattCallback = object : BluetoothGattCallback() {

        @SuppressLint("MissingPermission")
        override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
            if (newState == BluetoothProfile.STATE_CONNECTED) {
                addLog("已連線，搜索服務...")
                gatt.requestMtu(256)
            } else {
                _state.value = ConnectionState.Disconnected
                addLog("連線中斷 (status=$status)")
            }
        }

        @SuppressLint("MissingPermission")
        override fun onMtuChanged(gatt: BluetoothGatt, mtu: Int, status: Int) {
            addLog("MTU=$mtu")
            gatt.discoverServices()
        }

        @SuppressLint("MissingPermission")
        override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
            val service = gatt.getService(NUS_SERVICE_UUID)
            if (service == null) {
                addLog("找不到 NUS 服務！")
                gatt.disconnect()
                return
            }
            rxChar = service.getCharacteristic(NUS_RX_UUID)
            val txChar = service.getCharacteristic(NUS_TX_UUID)
            if (rxChar == null || txChar == null) {
                addLog("找不到 NUS characteristics！")
                gatt.disconnect()
                return
            }

            // Enable TX notifications
            gatt.setCharacteristicNotification(txChar, true)
            val cccd = txChar.getDescriptor(UUID.fromString(CCCD_UUID_STR))
            cccd?.let {
                it.value = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
                gatt.writeDescriptor(it)
            }
            _state.value = ConnectionState.Connected
            addLog("NUS 服務就緒，請輸入密碼")
        }

        @Deprecated("Deprecated in API 33")
        override fun onCharacteristicChanged(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic) {
            if (characteristic.uuid == NUS_TX_UUID) {
                val text = characteristic.value?.toString(Charsets.UTF_8)?.trim() ?: return
                addLog("← $text")
                _response.value = text
                pendingResponses.poll()?.complete(text)
            }
        }
    }

    // ── Send command ──

    @SuppressLint("MissingPermission")
    suspend fun send(command: String, timeoutMs: Long = 5000): String {
        val gatt = bluetoothGatt ?: return "ERR not connected"
        val rx = rxChar ?: return "ERR no RX characteristic"

        addLog("→ $command")
        val deferred = CompletableDeferred<String>()
        pendingResponses.add(deferred)

        rx.writeType = BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
        rx.value = command.toByteArray(Charsets.UTF_8)
        gatt.writeCharacteristic(rx)

        return try {
            withTimeout(timeoutMs) { deferred.await() }
        } catch (e: TimeoutCancellationException) {
            pendingResponses.remove(deferred)
            "ERR timeout"
        }
    }

    // ── Auth ──

    suspend fun authenticate(pin: String): Boolean {
        val resp = send("AUTH $pin")
        if (resp == "OK") {
            _state.value = ConnectionState.Authenticated
            addLog("認證成功 ✓")
            return true
        }
        addLog("認證失敗: $resp")
        return false
    }

    // ── Config helpers ──

    suspend fun getConfig(key: String): String {
        val resp = send("GET $key")
        return if (resp.startsWith("OK ")) resp.removePrefix("OK ").trim() else resp
    }

    suspend fun setConfig(key: String, value: String): Boolean {
        val resp = send("SET $key $value")
        return resp == "OK"
    }

    suspend fun listKeys(): List<String> {
        val resp = send("LIST")
        return resp.split(",").map { it.trim() }.filter { it.isNotEmpty() }
    }

    suspend fun saveAndRestart(): String {
        return send("SAVE")
    }

    suspend fun ping(): String {
        return send("PING")
    }
}
