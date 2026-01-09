package com.thejebforge.badgeproject.ui.view

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.ArrowBack
import androidx.compose.material.icons.filled.Brightness1
import androidx.compose.material.icons.filled.Brightness7
import androidx.compose.material.icons.outlined.Brightness1
import androidx.compose.material.icons.rounded.Bluetooth
import androidx.compose.material.icons.rounded.BluetoothConnected
import androidx.compose.material.icons.rounded.BluetoothDisabled
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
import com.thejebforge.badgeproject.data.intermediate.BoardMode
import com.thejebforge.badgeproject.service.BoardService
import com.thejebforge.badgeproject.data.intermediate.CharacterAction
import com.thejebforge.badgeproject.data.intermediate.CharacterState
import com.thejebforge.badgeproject.ui.theme.BadgeProjectTheme
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject

@HiltViewModel
class DeviceControlViewModel @Inject constructor() : ViewModel() {
    companion object : IRoute {
        override val name: String
            get() = "device_control"
    }

    var device: Device? = null
    var backlightUpdating: Boolean = false
}

@Composable
fun DeviceControlScreen(
    navigateBack: () -> Unit,
    viewModel: DeviceControlViewModel,
    service: BoardService?
) {
    DeviceControlScreenContent(
        viewModel.device,
        deviceConnected = service?.state?.deviceConnected?.value ?: false,
        navigateBack,
        service?.state?.backlight?.value ?: false,
        viewModel.backlightUpdating,
        {
            service?.run {
                viewModel.backlightUpdating = true
                toggleBacklight {
                    viewModel.backlightUpdating = false
                }
            }
        },
        service?.let {
            service ->
            UIDeviceMode.fromCharacterState(
                service.state.character.value
            ) {
                id ->
                service.invokeAction(id)
            }
        }
    )
}

sealed class UIDeviceMode(
    val mode: Int
) {
    data class Character(
        val name: String,
        val species: String,
        val actions: List<CharacterAction>,
        val runAction: (String) -> Unit
    ) : UIDeviceMode(
        R.string.character_mode
    )

    data object None : UIDeviceMode(R.string.no_devices_found)

    companion object {
        fun fromCharacterState(state: CharacterState, runAction: (String) -> Unit): UIDeviceMode? = when (state) {
            is CharacterState.Loaded if state.info.mode.value == BoardMode.CHARACTER ->
                with(state.info) {
                    Character(
                        name.value ?: "Loading...",
                        species.value ?: "Loading...",
                        actions.toList(),
                        runAction
                    )
                }
            is CharacterState.Loaded -> None
            CharacterState.Loading -> null
            CharacterState.Failed -> None
        }
    }
}

@Composable
fun DeviceControlScreenContent(
    device: Device?,
    deviceConnected: Boolean,
    navigateBack: () -> Unit,
    backlight: Boolean,
    backlightUpdating: Boolean,
    toggleBacklight: () -> Unit,
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
                            if (deviceConnected) {
                                Icons.Rounded.BluetoothConnected
                            } else {
                                Icons.Rounded.BluetoothDisabled
                            },
                            "Connection Status"
                        )

                        Column {
                            Text(
                                device?.name ?: "Missing",
                                style = MaterialTheme.typography.bodyLarge
                            )
                            Text(
                                device?.mac ?: "Missing",
                                style = MaterialTheme.typography.bodySmall
                            )
                        }
                    }
                }

                IconButton(
                    onClick = toggleBacklight
                ) {
                    if (backlightUpdating) {
                        CircularProgressIndicator(
                            Modifier.padding(5.dp),
                            color = MaterialTheme.colorScheme.primary,
                            strokeWidth = 3.dp
                        )
                        return@IconButton
                    }

                    Icon(
                        if (backlight)
                            Icons.Default.Brightness7
                        else
                            Icons.Outlined.Brightness1,
                        "Device Backlight"
                    )
                }
            }

            if (!deviceConnected) {
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

            if (modeInfo == null) {
                Surface(
                    color = MaterialTheme.colorScheme.surfaceContainer,
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text(
                        stringResource(R.string.info_loading),
                        style = MaterialTheme.typography.headlineLarge,
                        modifier = Modifier.fillMaxWidth().padding(10.dp, 40.dp),
                        textAlign = TextAlign.Center
                    )
                }
                return@Column
            }

            if (modeInfo == UIDeviceMode.None) {
                Surface(
                    color = MaterialTheme.colorScheme.errorContainer,
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text(
                        stringResource(R.string.info_failed_to_load),
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
                                        modeInfo.runAction(action.id)
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
                                        action.name,
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
            navigateBack = {},
            backlight = true,
            backlightUpdating = false,
            toggleBacklight = {},
            modeInfo = UIDeviceMode.Character(
                "Test Character",
                "Some species",
                listOf(
                    CharacterAction("test", "What")
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
            navigateBack = {},
            backlight = true,
            backlightUpdating = true,
            toggleBacklight = {},
            modeInfo = UIDeviceMode.Character(
                "Test Character",
                "Some species",
                listOf(
                    CharacterAction("test", "What")
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
private fun PreviewNotLoaded() {
    BadgeProjectTheme {
        DeviceControlScreenContent(
            Device("BP Board", "00:1A:2B:3C:4D:5E"),
            deviceConnected = true,
            navigateBack = {},
            backlight = false,
            backlightUpdating = false,
            toggleBacklight = {},
            modeInfo = null
        )
    }
}

@Preview(
    showSystemUi = true
)
@Composable
private fun PreviewFailed() {
    BadgeProjectTheme {
        DeviceControlScreenContent(
            Device("BP Board", "00:1A:2B:3C:4D:5E"),
            deviceConnected = true,
            navigateBack = {},
            backlight = false,
            backlightUpdating = false,
            toggleBacklight = {},
            modeInfo = UIDeviceMode.None
        )
    }
}