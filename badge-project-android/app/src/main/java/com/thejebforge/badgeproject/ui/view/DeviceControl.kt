package com.thejebforge.badgeproject.ui.view

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import com.thejebforge.badgeproject.R
import com.thejebforge.badgeproject.data.intermediate.Device
import com.thejebforge.badgeproject.service.BoardService
import com.thejebforge.badgeproject.ui.theme.BadgeProjectTheme
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject

@HiltViewModel
class DeviceControlViewModel @Inject constructor() : ViewModel() {
    companion object : IRoute {
        override val name: String
            get() = "device_control"
    }
}

@Composable
fun DeviceControlScreen(
    device: Device,
    navigateBack: () -> Unit,
    viewModel: DeviceControlViewModel,
    service: BoardService?
) {
    DeviceControlScreenContent(
        device,
        deviceConnected = service?.currentDevice?.value != null,
        loading = false,
        navigateBack,
        UIDeviceMode.Character( // TODO: Replace with service stuff
            "Test Character",
            "Some species",
            listOf("huh", "what"),
            {}
        )
    )
}

sealed class UIDeviceMode(
    val mode: Int
) {
    data class Character(
        val name: String,
        val species: String,
        val actions: List<String>,
        val runAction: (String) -> Unit
    ) : UIDeviceMode(
        R.string.character_mode
    )

    data object None : UIDeviceMode(R.string.no_devices_found)
}

@Composable
fun DeviceControlScreenContent(
    device: Device,
    deviceConnected: Boolean,
    loading: Boolean,
    navigateBack: () -> Unit,
    modeInfo: UIDeviceMode?
) {
    Surface {
        Column(
            Modifier.fillMaxSize()
                .statusBarsPadding()
                .navigationBarsPadding()
                .padding(5.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            Row(
                Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                IconButton(
                    onClick = navigateBack
                ) {
                    Icon(
                        Icons.AutoMirrored.Rounded.ArrowBack,
                        "Back"
                    )
                }

                Text (
                    stringResource(R.string.device_control),
                    modifier = Modifier.weight(1f),
                    style = MaterialTheme.typography.headlineSmall
                )

                Surface(
                    color = if (deviceConnected) {
                        MaterialTheme.colorScheme.primaryContainer
                    } else {
                        MaterialTheme.colorScheme.errorContainer
                    },
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Row(
                        Modifier.padding(7.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(7.dp)
                    ) {
                        Icon(
                            painterResource(if (deviceConnected) {
                                R.drawable.bluetooth_connected_24px
                            } else {
                                R.drawable.bluetooth_disabled_24px
                            }),
                            "Connection Status"
                        )

                        Column {
                            Text(
                                device.name,
                                style = MaterialTheme.typography.bodyLarge
                            )
                            Text(
                                device.mac,
                                style = MaterialTheme.typography.bodySmall
                            )
                        }
                    }
                }
            }

            if (!deviceConnected || modeInfo == null) {
                Surface(
                    color = MaterialTheme.colorScheme.errorContainer,
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text(
                        stringResource(R.string.device_disconnected),
                        style = MaterialTheme.typography.headlineLarge,
                        modifier = Modifier.fillMaxWidth().padding(10.dp, 40.dp),
                        textAlign = TextAlign.Center
                    )
                }
                return@Column
            }

            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                Card {
                    Column(
                        Modifier.padding(10.dp)
                    ) {
                        Text(
                            stringResource(R.string.current_mode),
                            style = MaterialTheme.typography.labelMedium
                        )
                        Text(
                            stringResource(modeInfo.mode),
                            style = MaterialTheme.typography.bodyLarge
                        )
                    }
                }

                when (modeInfo) {
                    is UIDeviceMode.Character -> {
                        Card {
                            Column(
                                Modifier.padding(10.dp)
                            ) {
                                Text(
                                    stringResource(R.string.character_name),
                                    style = MaterialTheme.typography.labelMedium
                                )
                                Text(
                                    modeInfo.name,
                                    style = MaterialTheme.typography.bodyLarge
                                )
                            }
                        }

                        Card {
                            Column(
                                Modifier.padding(10.dp)
                            ) {
                                Text(
                                    stringResource(R.string.character_species),
                                    style = MaterialTheme.typography.labelMedium
                                )
                                Text(
                                    modeInfo.species,
                                    style = MaterialTheme.typography.bodyLarge
                                )
                            }
                        }
                    }

                    else -> {}
                }
            }

            when (modeInfo) {
                is UIDeviceMode.Character -> {
                    LazyColumn(
                        verticalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        items(modeInfo.actions) { action ->
                            Surface(
                                Modifier.fillMaxWidth()
                                    .clickable {
                                        modeInfo.runAction(action)
                                    },
                                color = MaterialTheme.colorScheme.primary,
                                shape = RoundedCornerShape(10.dp)
                            ) {
                                Column(
                                    Modifier.padding(15.dp),
                                ) {
                                    Text(
                                        stringResource(R.string.run_action),
                                        style = MaterialTheme.typography.labelMedium
                                    )

                                    Text(
                                        action,
                                        style = MaterialTheme.typography.headlineMedium
                                    )
                                }
                            }
                        }
                    }
                }

                else -> {}
            }
        }
    }
}

@Preview(
    showSystemUi = true
)
@Composable
private fun Preview() {
    BadgeProjectTheme {
        DeviceControlScreenContent(
            Device("BP Board", "00:1A:2B:3C:4D:5E"),
            deviceConnected = true,
            loading = false,
            navigateBack = {},
            modeInfo = UIDeviceMode.Character(
                "Test Character",
                "Some species",
                listOf(
                    "huh", "what", "yes", "wow", "more actions", "even more garbage", "yessss", "fill the whole screen!",
                    "huh", "what", "yes", "wow", "more actions", "even more garbage", "yessss", "fill the whole screen!"
                ),
                {}
            )
        )
    }
}

@Preview(
    showSystemUi = true
)
@Composable
private fun PreviewNotConnected() {
    BadgeProjectTheme {
        DeviceControlScreenContent(
            Device("BP Board", "00:1A:2B:3C:4D:5E"),
            deviceConnected = false,
            loading = false,
            navigateBack = {},
            modeInfo = UIDeviceMode.Character(
                "Test Character",
                "Some species",
                listOf("huh", "what"),
                {}
            )
        )
    }
}