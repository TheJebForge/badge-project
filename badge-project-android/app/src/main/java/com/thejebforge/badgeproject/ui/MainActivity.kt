package com.thejebforge.badgeproject.ui

import android.Manifest
import android.annotation.SuppressLint
import android.app.ComponentCaller
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.Bundle
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.hilt.lifecycle.viewmodel.compose.hiltViewModel
import androidx.navigation.NavController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.thejebforge.badgeproject.data.intermediate.Device
import com.thejebforge.badgeproject.service.BoardService
import com.thejebforge.badgeproject.service.BoardServiceImpl
import com.thejebforge.badgeproject.ui.theme.BadgeProjectTheme
import com.thejebforge.badgeproject.ui.view.*
import com.thejebforge.badgeproject.util.BluetoothHelper
import com.thejebforge.badgeproject.util.PermissionsHelper
import dagger.hilt.android.AndroidEntryPoint
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonClassDiscriminator
import javax.inject.Inject

@AndroidEntryPoint
class MainActivity @Inject constructor() : ComponentActivity() {
    private var navController: NavController? = null
    private var permissionLauncher: ActivityResultLauncher<Array<String>>? = null
    private var service: MutableState<BoardService?> = mutableStateOf(null)
    private var serviceConnectedCallback: ((Boolean) -> Unit)? = null
    private val handler = Handler(Looper.getMainLooper())

    private val serviceConnection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName, service: IBinder) {
            val binder = service as BoardService.BoardServiceBinder
            this@MainActivity.service.value = binder.getService()
            binder.stopAction = ::unbindBoard
            binder.connectedAction = serviceConnectedCallback
            Log.i(MainActivity::class.simpleName, "Bound to the service!")

            if (this@MainActivity.service.value?.state?.deviceConnected?.value == true) {
                serviceConnectedCallback?.invoke(true)
            }
        }

        override fun onServiceDisconnected(name: ComponentName) {
            service.value = null
        }
    }

    @Inject
    lateinit var bluetoothHelper: BluetoothHelper

    val permissionsToRequest = arrayOf(
        Manifest.permission.BLUETOOTH_SCAN,
        Manifest.permission.BLUETOOTH_CONNECT,
        Manifest.permission.POST_NOTIFICATIONS
    )

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // Deal with runtime permissions
        permissionLauncher = registerForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) {
            permsResult ->
            for (perm in permissionsToRequest) {
                permsResult[perm]?.let {
                    if (!it) {
                        showPermissionsRationale()
                        return@registerForActivityResult
                    }
                }
            }
            permissionsGranted(true)
        }

        setContent {
            val navController = rememberNavController()
            this.navController = navController

            BadgeProjectTheme {
                NavHost(
                    navController = navController,
                    startDestination = WaitingForPermissions.name,
                    enterTransition = {
                        slideInHorizontally { fullWidth -> fullWidth }
                    },
                    exitTransition = {
                        slideOutHorizontally { fullWidth -> -fullWidth }
                    },
                    popEnterTransition = {
                        slideInHorizontally { fullWidth -> -fullWidth }
                    },
                    popExitTransition = {
                        slideOutHorizontally { fullWidth -> fullWidth }
                    }
                ) {
                    composable(WaitingForPermissions.name) {
                        WaitingForPermissionsScreen {
                            when {
                                PermissionsHelper.gotPermissions(this@MainActivity, permissionsToRequest) -> {
                                    permissionsGranted()
                                }
                                PermissionsHelper.shouldShowRationale(this@MainActivity, permissionsToRequest) -> {
                                    showPermissionsRationale()
                                }
                                else -> {
                                    permissionLauncher?.launch(permissionsToRequest)
                                }
                            }
                        }
                    }
                    composable(NoPermissions.name) {
                        NoPermissionsScreen {
                            permissionLauncher?.launch(permissionsToRequest)
                        }
                    }
                    composable(DeviceListViewModel.name) {
                        val viewModel = hiltViewModel<DeviceListViewModel>()

                        viewModel.scanDevicesAction = @SuppressLint("MissingPermission") {
                            callback ->
                            bluetoothHelper.scanForDevices(callback)
                        }
                        viewModel.accessActivityAction = {
                            callback -> callback(this@MainActivity)
                        }
                        viewModel.connectDeviceAction = {
                            device, callback ->
                            serviceConnectedCallback = {
                                success ->
                                mainExecutor.execute {
                                    callback(success)

                                    if (success) {
                                        if (navController.currentDestination?.route != DeviceControlViewModel.name) {
                                            navController.navigate(DeviceControlViewModel.name)
                                        }
                                    }
                                }
                            }

                            service.value?.binder?.connectedAction = serviceConnectedCallback

                            Intent(
                                this@MainActivity,
                                BoardServiceImpl::class.java
                            ).apply {
                                action = BoardService.StartAction.Connect(device).serialize()
                            }.also {
                                bindService(it, serviceConnection, BIND_AUTO_CREATE)
                                startService(it)
                            }
                        }

                        DeviceListScreen(
                            viewModel,
                            service.value?.state?.currentDevice?.value,
                            service.value?.state?.deviceConnected?.value ?: false
                        )
                    }
                    composable(DeviceControlViewModel.name) {
                        val viewModel = hiltViewModel<DeviceControlViewModel>()

                        service.value?.state?.currentDevice?.value?.let {
                            viewModel.device = it.copy()
                        }

                        DeviceControlScreen(
                            {
                                navController.popBackStack()
                            },
                            viewModel,
                            service.value
                        )
                    }
                }
            }
        }
    }

    @OptIn(ExperimentalSerializationApi::class)
    @JsonClassDiscriminator("type")
    @Serializable
    sealed class StartAction {
        @Serializable
        @SerialName("open_device")
        data class OpenDevice(val device: Device) : StartAction()

        fun serialize(): String = Json.encodeToString<StartAction>(this)
    }

    private fun tryGetStartAction(): StartAction? {
        try {
            val action = intent?.action?.let { Json.decodeFromString<StartAction>(it) }
            return action
        } catch (_: Exception) {
            return null
        }
    }

    override fun onResume() {
        super.onResume()

        when {
            PermissionsHelper.gotPermissions(this@MainActivity, permissionsToRequest) -> {
                permissionsGranted(true)
            }
            PermissionsHelper.shouldShowRationale(this@MainActivity, permissionsToRequest) -> {
                showPermissionsRationale()
            }
        }

        tryGetStartAction()?.let {
            when(it) {
                is StartAction.OpenDevice -> {
                    Log.i(MainActivity::class.simpleName, "Trying to open device control for ${it.device.name} (${it.device.mac})")
                    handler.postDelayed({
                        if (navController?.currentDestination?.route != DeviceControlViewModel.name) {
                            navController?.navigate(DeviceControlViewModel.name)
                        }
                    }, 200L)
                }
            }
        }

        Intent(
            this@MainActivity,
            BoardServiceImpl::class.java
        ).also {
            bindService(it, serviceConnection, 0)
        }
    }

    private fun unbindBoard() {
        if (service.value != null) {
            unbindService(serviceConnection)
            service.value = null
        }
    }

    override fun onStop() {
        super.onStop()
        unbindBoard()
    }

    private fun permissionsGranted(onlyIfWaiting: Boolean = false) {
        navController?.let {
            if (onlyIfWaiting) {
                if (it.currentDestination?.route != WaitingForPermissions.name
                    && it.currentDestination?.route != NoPermissions.name
                ) return@let
            }

            it.navigate(DeviceListViewModel.name) {
                popUpTo(it.graph.id) {
                    inclusive = true
                }
            }
        }
    }

    private fun showPermissionsRationale() {
        navController?.let {
            it.navigate(NoPermissions.name) {
                popUpTo(it.graph.id) {
                    inclusive = true
                }
            }
        }
    }
}