package com.thejebforge.badgeproject.service

import android.annotation.SuppressLint
import android.app.Notification
import android.app.NotificationManager
import android.app.PendingIntent
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.ServiceInfo
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.util.Log
import androidx.compose.runtime.mutableStateOf
import androidx.core.app.NotificationCompat
import com.thejebforge.badgeproject.MainApplication
import com.thejebforge.badgeproject.R
import com.thejebforge.badgeproject.data.intermediate.BoardMode
import com.thejebforge.badgeproject.data.intermediate.CharacterAction
import com.thejebforge.badgeproject.data.intermediate.CharacterInfo
import com.thejebforge.badgeproject.data.intermediate.CharacterState
import com.thejebforge.badgeproject.gatt.*
import com.thejebforge.badgeproject.gatt.command.*
import com.thejebforge.badgeproject.ui.MainActivity
import kotlinx.serialization.json.Json

@SuppressLint("MissingPermission")
class BoardServiceImpl : BoardService() {
    companion object {
        const val BOARD_NOTIFICATION_ID: Int = 1
        const val START_SERVICE_FLAG: Int = START_REDELIVER_INTENT
        private val TAG: String = BoardServiceImpl::class.simpleName!!
    }

    private var running: Boolean = false

    private val handler = Handler(Looper.getMainLooper())
    private lateinit var notificationManager: NotificationManager
    private lateinit var bluetoothManager: BluetoothManager
    private lateinit var bluetoothAdapter: BluetoothAdapter
    private var gatt: GATTHelper? = null

    private fun createNotification(): Notification = NotificationCompat.Builder(
        this,
        MainApplication.BOARD_NOTIFICATION_CHANNEL
    )
        .setSmallIcon(R.drawable.badge_project_notif)
        .setContentTitle(getString(R.string.app_name))
        .setContentText(getString(
            R.string.notification_desc,
            state.currentDevice.value?.name,
            state.currentDevice.value?.mac
        ))
        .setSilent(true)
        .setOngoing(true)
        .setLocalOnly(true)
        .setContentIntent(
            Intent(
                this,
                MainActivity::class.java
            ).apply {
                action = state.currentDevice.value?.let {
                    MainActivity.StartAction.OpenDevice(it).serialize()
                }
            }.let {
                    intent ->
                PendingIntent.getActivity(
                    this@BoardServiceImpl,
                    0,
                    intent,
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
                )
            }
        )
        .setDeleteIntent(
            Intent(
                this,
                BoardServiceImpl::class.java
            ).apply {
                action = StartAction.Stop.serialize()
            }.let {
                    intent ->
                PendingIntent.getService(
                    this@BoardServiceImpl,
                    0,
                    intent,
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
                )
            }
        )
        .build()

    override fun onCreate() {
        super.onCreate()
        notificationManager = getSystemService(NotificationManager::class.java)
        bluetoothManager = getSystemService(BluetoothManager::class.java)

        if (bluetoothManager.adapter == null) {
            throw IllegalStateException("No adapter??")
        }
        bluetoothAdapter = bluetoothManager.adapter

        registerReceiver(
            bondBroadcastReceiver,
            IntentFilter(BluetoothDevice.ACTION_BOND_STATE_CHANGED)
        )
    }

    override fun onDestroy() {
        super.onDestroy()
        unregisterReceiver(bondBroadcastReceiver)
    }

    private val bondBroadcastReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            val device = intent?.extras
                ?.getParcelable(BluetoothDevice.EXTRA_DEVICE, BluetoothDevice::class.java)
            val bondState = intent?.extras
                ?.getInt(BluetoothDevice.EXTRA_BOND_STATE)

            if (device == null || device.address != state.currentDevice.value?.mac) return

            when (bondState) {
                BluetoothDevice.BOND_BONDING -> {
                    Log.i(TAG, "Android is binding with the device...")
                }

                BluetoothDevice.BOND_BONDED -> {
                    Log.i(TAG, "Successfully bonded!")
                    actuallyConnect(device)
                }

                BluetoothDevice.BOND_NONE -> {
                    Log.e(TAG, "Failed to bond with the device or device was forcefully unbonded!")
                    stop()
                }
            }
        }
    }

    fun updateBoardInfo() {
        gatt?.let {
                gatt ->
            gatt.getDeviceMode {
                    maybeMode ->
                if (maybeMode == null) {
                    state.character.value = CharacterState.Failed
                    return@getDeviceMode
                }

                val mode = BoardMode.fromInt(maybeMode)
                if (mode == null) {
                    state.character.value = CharacterState.Failed
                    return@getDeviceMode
                }

                val charInfo = CharacterInfo(
                    mode = mutableStateOf(mode)
                )

                gatt.getCharacterId {
                    charInfo.id.value = it
                }.getCharacterName {
                    charInfo.name.value = it
                }.getCharacterSpecies {
                    charInfo.species.value = it
                }.getActionList {
                        receivedList ->
                    if (receivedList == null) return@getActionList

                    charInfo.actions.let {
                            actionList ->
                        actionList.clear()

                        for (action in receivedList) {
                            val (id, name) = action.getOrNull() ?: continue
                            actionList.add(CharacterAction(id, name))
                        }
                    }
                }.getCharacterList {
                        receivedList ->
                    if (receivedList == null) return@getCharacterList

                    state.characterList.clear()

                    for (character in receivedList) {
                        val id = character.getOrNull() ?: continue
                        state.characterList.add(id)
                    }
                }.getBacklightState {
                    if (it == null) return@getBacklightState

                    state.backlight.value = it
                }

                state.character.value = CharacterState.Loaded(charInfo)
            }
        }
    }

    private fun actuallyConnect(device: BluetoothDevice) {
        Log.i(TAG, "Connecting to ${device.address}")

        GATTHelper.connect(
            this,
            handler,
            device
        ) { (gatt, connected) ->
            if (connected) {
                Log.i(TAG, "Device connected!")
                state.deviceConnected.value = true
                state.hadConnection.value = true
                binder.connectedAction?.invoke(true)

                gatt.discoverServices {
                    Log.i(TAG, "Discovered device services...")
                }.enableCommands {
                    Log.i(TAG, "Commands enabled...")
                    updateBoardInfo()
                }

                Log.i(TAG, "All commands were sent")
            } else {
                Log.i(TAG, "Device disconnected!")
                state.deviceConnected.value = false
                binder.connectedAction?.invoke(false)

                if (!state.hadConnection.value) {
                    stop()
                } else {
                    if (running) {
                        Log.i(TAG, "Attempting to reconnect...")
                        actuallyConnect(device)
                    }
                }
            }
        }.onSuccess {
            gatt = it
        }.onFailure {
            Log.e(TAG, "Failed to connect GATT! $it")
            stop()
        }
    }

    override fun invokeAction(id: String) {
        gatt?.invokeAction(id) {}
    }

    override fun toggleBacklight(onDone: () -> Unit) {
        val newBacklight = !state.backlight.value
        gatt?.setBacklightState(newBacklight) {
            if (it) {
                state.backlight.value = newBacklight
            }
            onDone()
        }
    }

    override fun switchCharacter(id: String) {
        Log.i(TAG, "Trying to switch to $id")
        gatt?.switchCharacter(id) {
            handler.postDelayed({
                updateBoardInfo()
            }, 100L)
        }
    }

    private fun start(intent: Intent?): Boolean {
        running = true

        val action = intent?.action?.let { Json.decodeFromString<StartAction>(it) }
        if (action == null) {
            Log.e(TAG, "Received invalid intent action!")
            stop()
            return false
        }

        when(action) {
            is StartAction.Connect -> {
                val device = action.device

                if (device == state.currentDevice.value) {
                    Log.i(TAG, "Already connected, skipping connection process")
                    handler.postDelayed({
                        binder.connectedAction?.invoke(true)
                    }, 100)
                    return true
                }

                gatt?.run {
                    disconnect()
                    gatt = null
                }

                state.currentDevice.value = device

                try {
                    val device = bluetoothAdapter.getRemoteDevice(device.mac)
                    if (device.bondState != BluetoothDevice.BOND_BONDED) {
                        Log.i(TAG, "Trying to bond with the device...")

                        if (bluetoothAdapter.startDiscovery()) {
                            handler.postDelayed({
                                if (!device.createBond()) {
                                    Log.w(TAG, "Couldn't start bonding!")
                                    stop()
                                }
                                bluetoothAdapter.cancelDiscovery()
                            }, 2000)
                        } else {
                            Log.w(TAG, "Couldn't start discovery!")
                        }
                    } else {
                        actuallyConnect(device)
                    }
                } catch (_: IllegalArgumentException) {
                    Log.w(TAG, "Device doesn't exist!")
                    return false
                }

                startForeground(
                    BOARD_NOTIFICATION_ID,
                    createNotification(),
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE
                )
            }

            is StartAction.Stop -> {
                Log.i(TAG, "Stop action was called")
                stop()
            }
        }

        return true
    }

    private fun stop() {
        running = false
        binder.stopAction?.invoke()
        gatt?.run {
            disconnect()
            gatt = null
        }
        stopSelf()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (!start(intent)) stop()
        return START_SERVICE_FLAG
    }

    override fun onBind(intent: Intent?): IBinder {
        return binder
    }
}