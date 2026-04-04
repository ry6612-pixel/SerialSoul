package com.serialsoul.bleconfig

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {

    private lateinit var bleManager: BleManager

    private val permLauncher = registerForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { grants ->
        if (grants.values.all { it }) {
            bleManager.startScan()
        } else {
            Toast.makeText(this, "需要藍牙和定位權限才能掃描", Toast.LENGTH_LONG).show()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        bleManager = BleManager(this)

        setContent {
            MaterialTheme(
                colorScheme = darkColorScheme(
                    primary = Color(0xFF4FC3F7),
                    secondary = Color(0xFF81C784),
                    background = Color(0xFF121212),
                    surface = Color(0xFF1E1E1E),
                    onPrimary = Color.Black,
                    onBackground = Color.White,
                    onSurface = Color.White,
                )
            ) {
                SerialSoulApp(bleManager, ::requestPermAndScan)
            }
        }
    }

    private fun requestPermAndScan() {
        val perms = mutableListOf<String>()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            perms.add(Manifest.permission.BLUETOOTH_SCAN)
            perms.add(Manifest.permission.BLUETOOTH_CONNECT)
        }
        perms.add(Manifest.permission.ACCESS_FINE_LOCATION)

        val needed = perms.filter {
            ContextCompat.checkSelfPermission(this, it) != PackageManager.PERMISSION_GRANTED
        }
        if (needed.isEmpty()) {
            bleManager.startScan()
        } else {
            permLauncher.launch(needed.toTypedArray())
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        bleManager.disconnect()
    }
}

// ============================================================
//  Root Composable
// ============================================================

@Composable
fun SerialSoulApp(ble: BleManager, onScan: () -> Unit) {
    val connState by ble.state.collectAsState()
    val scope = rememberCoroutineScope()

    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        when (connState) {
            is BleManager.ConnectionState.Disconnected,
            is BleManager.ConnectionState.Scanning -> ScanScreen(ble, onScan)
            is BleManager.ConnectionState.Connecting -> ConnectingScreen()
            is BleManager.ConnectionState.Connected -> AuthScreen(ble)
            is BleManager.ConnectionState.Authenticated -> ConfigScreen(ble)
        }
    }
}

// ============================================================
//  Scan Screen
// ============================================================

@Composable
fun ScanScreen(ble: BleManager, onScan: () -> Unit) {
    val connState by ble.state.collectAsState()
    val devices by ble.devices.collectAsState()
    val isScanning = connState is BleManager.ConnectionState.Scanning

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp)
    ) {
        Text(
            "SerialSoul",
            fontSize = 28.sp,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.primary
        )
        Text("BLE 裝置設定", fontSize = 14.sp, color = Color.Gray)
        Spacer(modifier = Modifier.height(24.dp))

        Button(
            onClick = {
                if (isScanning) ble.stopScan() else onScan()
            },
            modifier = Modifier.fillMaxWidth(),
            colors = ButtonDefaults.buttonColors(
                containerColor = if (isScanning) Color(0xFFEF5350) else MaterialTheme.colorScheme.primary
            )
        ) {
            Icon(
                if (isScanning) Icons.Default.Close else Icons.Default.Search,
                contentDescription = null,
                modifier = Modifier.size(20.dp)
            )
            Spacer(modifier = Modifier.width(8.dp))
            Text(if (isScanning) "停止掃描" else "掃描 BLE 裝置")
        }

        if (isScanning && devices.isEmpty()) {
            Spacer(modifier = Modifier.height(32.dp))
            CircularProgressIndicator(modifier = Modifier.align(Alignment.CenterHorizontally))
            Spacer(modifier = Modifier.height(8.dp))
            Text("搜尋 SerialSoul 裝置中...", modifier = Modifier.align(Alignment.CenterHorizontally), color = Color.Gray)
        }

        Spacer(modifier = Modifier.height(16.dp))

        LazyColumn {
            items(devices) { dev ->
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .clickable { ble.connect(dev.device) },
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
                ) {
                    Row(
                        modifier = Modifier
                            .padding(16.dp)
                            .fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(
                            Icons.Default.Bluetooth,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(32.dp)
                        )
                        Spacer(modifier = Modifier.width(12.dp))
                        Column(modifier = Modifier.weight(1f)) {
                            Text(dev.name, fontWeight = FontWeight.Bold, fontSize = 16.sp)
                            Text(dev.address, fontSize = 12.sp, color = Color.Gray)
                        }
                        Text("${dev.rssi} dBm", fontSize = 12.sp, color = Color.Gray)
                    }
                }
            }
        }
    }
}

// ============================================================
//  Connecting Screen
// ============================================================

@Composable
fun ConnectingScreen() {
    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            CircularProgressIndicator(modifier = Modifier.size(48.dp))
            Spacer(modifier = Modifier.height(16.dp))
            Text("正在連線...", fontSize = 18.sp, color = Color.Gray)
        }
    }
}

// ============================================================
//  Auth Screen
// ============================================================

@Composable
fun AuthScreen(ble: BleManager) {
    var pin by remember { mutableStateOf("") }
    var showPin by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf("") }
    var loading by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Card(
            modifier = Modifier
                .padding(32.dp)
                .fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
        ) {
            Column(
                modifier = Modifier.padding(24.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Icon(
                    Icons.Default.Lock,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(48.dp)
                )
                Spacer(modifier = Modifier.height(16.dp))
                Text("裝置認證", fontSize = 20.sp, fontWeight = FontWeight.Bold)
                Text("請輸入 BLE 密碼", fontSize = 14.sp, color = Color.Gray)
                Spacer(modifier = Modifier.height(24.dp))

                OutlinedTextField(
                    value = pin,
                    onValueChange = { pin = it; error = "" },
                    label = { Text("密碼") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    visualTransformation = if (showPin) VisualTransformation.None else PasswordVisualTransformation(),
                    trailingIcon = {
                        IconButton(onClick = { showPin = !showPin }) {
                            Icon(
                                if (showPin) Icons.Default.VisibilityOff else Icons.Default.Visibility,
                                contentDescription = null
                            )
                        }
                    }
                )

                if (error.isNotEmpty()) {
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(error, color = Color(0xFFEF5350), fontSize = 13.sp)
                }

                Spacer(modifier = Modifier.height(16.dp))

                Button(
                    onClick = {
                        if (pin.isEmpty()) { error = "請輸入密碼"; return@Button }
                        loading = true
                        scope.launch {
                            val ok = ble.authenticate(pin)
                            loading = false
                            if (!ok) error = "密碼錯誤"
                        }
                    },
                    modifier = Modifier.fillMaxWidth(),
                    enabled = !loading
                ) {
                    if (loading) CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
                    else Text("登入")
                }

                Spacer(modifier = Modifier.height(8.dp))
                TextButton(onClick = { ble.disconnect() }) {
                    Text("斷線", color = Color.Gray)
                }
            }
        }
    }
}

// ============================================================
//  Config Screen (main settings editor)
// ============================================================

data class ConfigField(
    val key: String,
    val label: String,
    val icon: @Composable () -> Unit,
    val isSecret: Boolean = false,
    val hint: String = ""
)

@Composable
fun ConfigScreen(ble: BleManager) {
    val scope = rememberCoroutineScope()
    val logLines by ble.log.collectAsState()
    var showLog by remember { mutableStateOf(false) }
    var fwVersion by remember { mutableStateOf("") }
    var saving by remember { mutableStateOf(false) }
    var statusMsg by remember { mutableStateOf("") }

    // Config fields
    val fields = remember {
        listOf(
            ConfigField("wifi_ssid",   "WiFi SSID",        { Icon(Icons.Default.Wifi, null) }),
            ConfigField("wifi_pass",   "WiFi 密碼",         { Icon(Icons.Default.WifiPassword, null) }, isSecret = true),
            ConfigField("wifi_ssid2",  "WiFi SSID 2",      { Icon(Icons.Default.Wifi, null) }),
            ConfigField("wifi_pass2",  "WiFi 密碼 2",       { Icon(Icons.Default.WifiPassword, null) }, isSecret = true),
            ConfigField("tg_token",    "Telegram Token",    { Icon(Icons.Default.Send, null) }, isSecret = true, hint = "BotFather 給的 Token"),
            ConfigField("chat_id",     "Chat ID",           { Icon(Icons.Default.Person, null) }, hint = "你的 Telegram 用戶 ID"),
            ConfigField("gemini_key",  "Gemini API Key",    { Icon(Icons.Default.Key, null) }, isSecret = true, hint = "AIza..."),
            ConfigField("model_pref",  "AI 模型",           { Icon(Icons.Default.Psychology, null) }, hint = "如 gemini-2.0-flash"),
            ConfigField("voice_mode",  "語音模式",           { Icon(Icons.Default.RecordVoiceOver, null) }, hint = "tts / input / full / off"),
            ConfigField("wake_phrase", "喚醒詞",             { Icon(Icons.Default.Mic, null) }, hint = "如 ethan"),
            ConfigField("ble_pin",    "BLE 連線密碼",       { Icon(Icons.Default.Lock, null) }, isSecret = true, hint = "至少 6 字元"),
        )
    }

    val values = remember { mutableStateMapOf<String, String>() }
    val modified = remember { mutableStateMapOf<String, Boolean>() }

    // Load current values on first composition
    LaunchedEffect(Unit) {
        fwVersion = ble.ping()
        for (f in fields) {
            val v = ble.getConfig(f.key)
            values[f.key] = v
        }
    }

    Column(modifier = Modifier.fillMaxSize()) {
        // Top bar
        Surface(
            color = MaterialTheme.colorScheme.surface,
            tonalElevation = 4.dp
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(Icons.Default.Bluetooth, null, tint = MaterialTheme.colorScheme.primary)
                Spacer(modifier = Modifier.width(8.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text("SerialSoul 設定", fontWeight = FontWeight.Bold, fontSize = 18.sp)
                    Text(
                        fwVersion.ifEmpty { "已連線" },
                        fontSize = 12.sp,
                        color = Color.Gray
                    )
                }
                IconButton(onClick = { showLog = !showLog }) {
                    Icon(Icons.Default.Terminal, null, tint = Color.Gray)
                }
                IconButton(onClick = { ble.disconnect() }) {
                    Icon(Icons.Default.Close, null, tint = Color(0xFFEF5350))
                }
            }
        }

        if (showLog) {
            // Log panel
            Surface(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(160.dp),
                color = Color(0xFF0D1117)
            ) {
                LazyColumn(modifier = Modifier.padding(8.dp)) {
                    items(logLines) { line ->
                        Text(
                            line,
                            fontSize = 11.sp,
                            fontFamily = FontFamily.Monospace,
                            color = if (line.startsWith("←")) Color(0xFF81C784)
                                    else if (line.startsWith("→")) Color(0xFF4FC3F7)
                                    else Color(0xFFB0BEC5)
                        )
                    }
                }
            }
        }

        // Status message
        if (statusMsg.isNotEmpty()) {
            Surface(
                modifier = Modifier.fillMaxWidth(),
                color = if (statusMsg.startsWith("✓")) Color(0xFF1B5E20) else Color(0xFFB71C1C)
            ) {
                Text(
                    statusMsg,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                    color = Color.White,
                    fontSize = 13.sp
                )
            }
        }

        // Config fields
        Column(
            modifier = Modifier
                .weight(1f)
                .verticalScroll(rememberScrollState())
                .padding(16.dp)
        ) {
            fields.forEach { field ->
                ConfigFieldEditor(
                    field = field,
                    value = values[field.key] ?: "",
                    isModified = modified[field.key] == true,
                    onValueChange = { newVal ->
                        values[field.key] = newVal
                        modified[field.key] = true
                    }
                )
                Spacer(modifier = Modifier.height(12.dp))
            }
            Spacer(modifier = Modifier.height(80.dp))
        }

        // Bottom action bar
        Surface(
            color = MaterialTheme.colorScheme.surface,
            tonalElevation = 8.dp
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                OutlinedButton(
                    onClick = {
                        scope.launch {
                            statusMsg = ""
                            for (f in fields) {
                                values[f.key] = ble.getConfig(f.key)
                            }
                            modified.clear()
                            statusMsg = "✓ 已重新讀取"
                        }
                    },
                    modifier = Modifier.weight(1f)
                ) {
                    Icon(Icons.Default.Refresh, null, modifier = Modifier.size(18.dp))
                    Spacer(modifier = Modifier.width(4.dp))
                    Text("重讀")
                }

                Button(
                    onClick = {
                        saving = true
                        scope.launch {
                            statusMsg = ""
                            var ok = true
                            for ((key, isModified) in modified) {
                                if (isModified) {
                                    val v = values[key] ?: continue
                                    if (!ble.setConfig(key, v)) {
                                        statusMsg = "✗ 寫入 $key 失敗"
                                        ok = false
                                        break
                                    }
                                }
                            }
                            if (ok) {
                                val modifiedCount = modified.count { it.value }
                                if (modifiedCount > 0) {
                                    statusMsg = "✓ 已寫入 $modifiedCount 項，重啟裝置..."
                                    ble.saveAndRestart()
                                } else {
                                    statusMsg = "沒有修改的項目"
                                }
                            }
                            saving = false
                            modified.clear()
                        }
                    },
                    modifier = Modifier.weight(1f),
                    enabled = !saving && modified.any { it.value },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = Color(0xFF2E7D32)
                    )
                ) {
                    if (saving) {
                        CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp, color = Color.White)
                    } else {
                        Icon(Icons.Default.Save, null, modifier = Modifier.size(18.dp))
                    }
                    Spacer(modifier = Modifier.width(4.dp))
                    Text("儲存並重啟")
                }
            }
        }
    }
}

// ============================================================
//  Config Field Editor
// ============================================================

@Composable
fun ConfigFieldEditor(
    field: ConfigField,
    value: String,
    isModified: Boolean,
    onValueChange: (String) -> Unit
) {
    var showSecret by remember { mutableStateOf(false) }

    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text(field.label) },
        placeholder = { if (field.hint.isNotEmpty()) Text(field.hint, color = Color.Gray.copy(alpha = 0.5f)) },
        modifier = Modifier.fillMaxWidth(),
        singleLine = true,
        leadingIcon = field.icon,
        visualTransformation = if (field.isSecret && !showSecret) PasswordVisualTransformation() else VisualTransformation.None,
        trailingIcon = {
            Row {
                if (isModified) {
                    Icon(
                        Icons.Default.Edit,
                        contentDescription = null,
                        tint = Color(0xFFFFA726),
                        modifier = Modifier.size(18.dp)
                    )
                }
                if (field.isSecret) {
                    IconButton(onClick = { showSecret = !showSecret }) {
                        Icon(
                            if (showSecret) Icons.Default.VisibilityOff else Icons.Default.Visibility,
                            contentDescription = null
                        )
                    }
                }
            }
        },
        colors = OutlinedTextFieldDefaults.colors(
            focusedBorderColor = if (isModified) Color(0xFFFFA726) else MaterialTheme.colorScheme.primary,
            unfocusedBorderColor = if (isModified) Color(0xFFFFA726).copy(alpha = 0.5f) else Color.Gray.copy(alpha = 0.3f)
        )
    )
}
