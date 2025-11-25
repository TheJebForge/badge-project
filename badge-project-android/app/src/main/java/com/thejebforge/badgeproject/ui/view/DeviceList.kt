package com.thejebforge.badgeproject.ui.view

import android.content.Context
import android.content.res.Configuration
import android.widget.Toast
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.KeyboardArrowRight
import androidx.compose.material.icons.rounded.Refresh
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.snapshots.SnapshotStateList
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.thejebforge.badgeproject.R
import com.thejebforge.badgeproject.data.entity.PreviousDevice
import com.thejebforge.badgeproject.data.intermediate.Device
import com.thejebforge.badgeproject.data.repository.BPRepository
import com.thejebforge.badgeproject.ui.theme.BadgeProjectTheme
import com.thejebforge.badgeproject.util.BluetoothHelper
import com.thejebforge.badgeproject.util.Response
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.launch
import javax.inject.Inject

data class UIDevice (
    val name: String,
    val mac: String,
    val previous: MutableState<Boolean>,
    val found: MutableState<Boolean>,
    val connecting: MutableState<Boolean> = mutableStateOf(false)
) {
    constructor(
        name: String,
        mac: String,
        previous: Boolean,
        found: Boolean,
        connecting: Boolean = false
    ) : this(
        name = name,
        mac = mac,
        previous = mutableStateOf(previous),
        found = mutableStateOf(found),
        connecting = mutableStateOf(connecting)
    )

    constructor(device: Device) : this(
        device.name,
        device.mac,
        previous = false,
        found = true
    )

    constructor(previous: PreviousDevice) : this(
        previous.name,
        previous.macAddress,
        previous = true,
        found = false
    )

    fun asDevice(): Device = Device(
        name = name,
        mac = mac
    )
}

@HiltViewModel
class DeviceListViewModel @Inject constructor(
    private val repo: BPRepository
) : ViewModel() {
    companion object : IRoute {
        override val name: String
            get() = "device_list"
    }

    var scanning: MutableState<Boolean> = mutableStateOf(false)
    var devices: SnapshotStateList<UIDevice> = mutableStateListOf()

    var scanDevicesAction: (((BluetoothHelper.DeviceScanResponse) -> Unit) -> Unit)? = null
    var accessActivityAction: (((Context) -> Unit) -> Unit)? = null
    var connectDeviceAction: ((Device, () -> Unit) -> Unit)? = null

    init {
        viewModelScope.launch {
            when (val resp = repo.getPreviousDevices()) {
                is Response.Success -> {
                    devices.addAll(
                        0,
                        resp.data.asIterable()
                            .map(::UIDevice)
                    )
                }

                else -> {}
            }
        }
    }

    fun scanForDevices() {
        scanDevicesAction?.invoke { response ->
            when(response) {
                is BluetoothHelper.DeviceScanResponse.AlreadyScanning -> {
                    accessActivityAction?.invoke {
                        context ->
                        Toast.makeText(
                            context,
                            context.getString(R.string.already_scanning),
                            Toast.LENGTH_SHORT
                        ).show()
                    }
                }

                is BluetoothHelper.DeviceScanResponse.ScanStarted -> {
                    scanning.value = true
                    devices.removeIf { !it.previous.value }
                }

                is BluetoothHelper.DeviceScanResponse.FoundDevices -> {
                    for (receivedDevice in response.devices) {
                        val existing = devices.find { device -> device.mac == receivedDevice.mac }

                        if (existing != null) {
                            if (!existing.previous.value) continue
                            existing.found.value = true
                        } else {
                            devices.add(UIDevice(receivedDevice))
                        }
                    }
                }

                is BluetoothHelper.DeviceScanResponse.ScanStopped -> {
                    scanning.value = false
                }

                else -> {}
            }
        }
    }

    fun connectDevice(device: UIDevice) {
        viewModelScope.launch {
            device.connecting.value = true
            when (val it = repo.getPreviousDevice(device.mac)) {
                is Response.Error -> {
                    device.connecting.value = false
                    accessActivityAction?.invoke {
                            activity ->
                        Toast.makeText(
                            activity,
                            "Failed to access database to check for previous device",
                            Toast.LENGTH_LONG
                        ).show()
                    }
                }

                is Response.Success -> {
                    if (it.data == null) {
                        repo.addPreviousDevice(PreviousDevice(
                            macAddress = device.mac,
                            name = device.name
                        ))
                    }
                    device.previous.value = true

                    connectDeviceAction?.invoke(device.asDevice()) {
                        device.connecting.value = false
                    }
                }

                else -> {}
            }
        }
    }
}

@Composable
fun DeviceListScreen(
    viewModel: DeviceListViewModel,
    servicedDevice: Device?
) {
    DeviceListScreenContent(
        viewModel.scanning.value,
        {
            viewModel.scanForDevices()
        },
        viewModel.devices,
        {
            viewModel.connectDevice(it)
        },
        servicedDevice
    )
}

@Composable
fun DeviceListScreenContent(
    scanning: Boolean,
    scanAction: () -> Unit,
    devices: List<UIDevice>,
    connectDeviceAction: (UIDevice) -> Unit,
    servicedDevice: Device?
) {
    Surface {
        Column(
            Modifier.fillMaxSize()
                .statusBarsPadding()
                .navigationBarsPadding()
        ){
            Row(
                Modifier
                    .fillMaxWidth()
                    .padding(10.dp, 10.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    stringResource(R.string.device_list_title),
                    Modifier.weight(1f)
                        .padding(10.dp, 0.dp),
                    style = MaterialTheme.typography.headlineSmall
                )

                if (scanning) {
                    CircularProgressIndicator(
                        Modifier.padding(5.dp),
                        color = MaterialTheme.colorScheme.primary,
                        strokeWidth = 3.dp
                    )
                    return@Row
                }

                FilledIconButton(
                    modifier = Modifier.size(50.dp),
                    onClick = scanAction
                ) {
                    Icon(
                        Icons.Rounded.Refresh,
                        "Refresh"
                    )
                }
            }
            Card(
                Modifier
                    .weight(1f)
                    .fillMaxWidth()
                    .padding(10.dp, 0.dp, 10.dp, 10.dp)
            ) {
                if (devices.isEmpty()) {
                    Column(
                        Modifier.fillMaxSize(),
                        verticalArrangement = Arrangement.Center
                    ) {
                        Text(
                            stringResource(R.string.no_devices_found),
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(0.dp, 50.dp),
                            style = MaterialTheme.typography.displaySmall,
                            textAlign = TextAlign.Center,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                    return@Card
                }

                LazyColumn(
                    Modifier
                        .fillMaxSize()
                        .padding(10.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    items(devices) { device ->
                        val connectedToThis = servicedDevice?.mac == device.mac

                        Surface(
                            modifier = Modifier
                                .fillMaxWidth()
                                .combinedClickable(
                                    onClick = {
                                        connectDeviceAction(device)
                                    },
                                    onLongClick = {

                                    }
                                ),
                            shape = RoundedCornerShape(10.dp),
                            color = if (device.found.value || connectedToThis) {
                                MaterialTheme.colorScheme.primary
                            } else {
                                MaterialTheme.colorScheme.secondary
                            }
                        ) {
                            Row (
                                Modifier.fillMaxWidth().padding(15.dp, 10.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Column(
                                    Modifier.weight(1f)
                                ) {
                                    Text(
                                        device.name,
                                        style = MaterialTheme.typography.headlineMedium
                                    )
                                    Text(
                                        device.mac,
                                        style = MaterialTheme.typography.bodySmall
                                    )
                                }

                                if (device.previous.value || connectedToThis) {
                                    Text(
                                        stringResource(if (connectedToThis) {
                                            R.string.currently_connected
                                        } else {
                                            R.string.previously_connected
                                        }),
                                        style = MaterialTheme.typography.labelMedium
                                    )
                                }

                                if (device.connecting.value) {
                                    CircularProgressIndicator(
                                        Modifier.size(36.dp)
                                            .padding(5.dp),
                                        color = MaterialTheme.colorScheme.inversePrimary,
                                        strokeWidth = 3.dp
                                    )
                                } else {
                                    Icon(
                                        Icons.AutoMirrored.Rounded.KeyboardArrowRight,
                                        "Connect",
                                        Modifier.size(36.dp)
                                    )
                                }
                            }

                        }
                    }
                }
            }
        }
    }
}

private fun previewDevices(): List<UIDevice> = listOf(
    UIDevice(
        "test0",
        "test mac 0",
        previous = true,
        found = true,
        connecting = true
    ),
    UIDevice(
        "test1",
        "test mac 1",
        previous = true,
        found = false
    ),
    UIDevice(
        "test2",
        "test mac 2",
        previous = false,
        found = true
    ),
    UIDevice(
        "test3",
        "test mac 3",
        previous = false,
        found = false
    )
)

@Preview(
    showSystemUi = true
)
@Composable
private fun Preview() {
    BadgeProjectTheme {
        DeviceListScreenContent(
            false,
            {},
            devices = previewDevices(),
            {},
            Device("test0", "test mac 0")
        )
    }
}

@Preview(
    showSystemUi = true,
    uiMode = Configuration.UI_MODE_NIGHT_YES
)
@Composable
private fun DarkPreview() {
    BadgeProjectTheme {
        DeviceListScreenContent(
            false,
            {},
            devices = previewDevices(),
            {},
            Device("test0", "test mac 0")
        )
    }
}