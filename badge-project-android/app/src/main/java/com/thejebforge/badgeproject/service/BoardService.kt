package com.thejebforge.badgeproject.service

import android.annotation.SuppressLint
import android.app.Notification
import android.app.NotificationManager
import android.app.PendingIntent
import com.thejebforge.badgeproject.R
import android.app.Service
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.ServiceInfo
import android.os.Binder
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.util.Log
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.core.app.NotificationCompat
import androidx.core.app.TaskStackBuilder
import com.thejebforge.badgeproject.MainApplication
import com.thejebforge.badgeproject.data.intermediate.Device
import com.thejebforge.badgeproject.ui.MainActivity
import com.thejebforge.badgeproject.util.GATTHelper
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonClassDiscriminator
import java.lang.IllegalArgumentException
import java.nio.charset.Charset
import java.util.UUID
import kotlin.uuid.Uuid

@SuppressLint("MissingPermission")
class BoardService : Service() {
    companion object {
        const val BOARD_NOTIFICATION_ID: Int = 1
        const val START_SERVICE_FLAG: Int = START_REDELIVER_INTENT
        private val TAG: String = BoardService::class.simpleName!!
    }

    val currentDevice: MutableState<Device?> = mutableStateOf(null)
    val deviceConnected: MutableState<Boolean> = mutableStateOf(false)

    private val handler = Handler(Looper.getMainLooper())
    private lateinit var notificationManager: NotificationManager
    private lateinit var bluetoothManager: BluetoothManager
    private lateinit var bluetoothAdapter: BluetoothAdapter
    private var gatt: GATTHelper? = null

    private fun createNotification(): Notification = NotificationCompat.Builder(
        this,
        MainApplication.BOARD_NOTIFICATION_CHANNEL
    )
        .setSmallIcon(R.drawable.ic_launcher_foreground)
        .setContentTitle(getString(R.string.app_name))
        .setContentText(getString(
            R.string.notification_desc,
            currentDevice.value?.name,
            currentDevice.value?.mac
        ))
        .setSilent(true)
        .setOngoing(true)
        .setLocalOnly(true)
        .setContentIntent(
            Intent(
                this,
                MainActivity::class.java
            ).apply {
                action = currentDevice.value?.let {
                    MainActivity.StartAction.OpenDevice(it).serialize()
                }
            }.let {
                intent ->
                PendingIntent.getActivity(
                    this@BoardService,
                    0,
                    intent,
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
                )
            }
        )
        .setDeleteIntent(
            Intent(
                this,
                BoardService::class.java
            ).apply {
                action = StartAction.Stop.serialize()
            }.let {
                intent ->
                PendingIntent.getService(
                    this@BoardService,
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

    @OptIn(ExperimentalSerializationApi::class)
    @JsonClassDiscriminator("type")
    @Serializable
    sealed class StartAction {
        @Serializable
        @SerialName("connect")
        data class Connect(val device: Device) : StartAction()

        @Serializable
        @SerialName("stop")
        data object Stop : StartAction()

        fun serialize(): String = Json.encodeToString<StartAction>(this)
    }

    private val bondBroadcastReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            val device = intent?.extras
                ?.getParcelable(BluetoothDevice.EXTRA_DEVICE, BluetoothDevice::class.java)
            val bondState = intent?.extras
                ?.getInt(BluetoothDevice.EXTRA_BOND_STATE)

            if (device == null || device.address != currentDevice.value?.mac) return

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

    private fun actuallyConnect(device: BluetoothDevice) {
        GATTHelper.connect(
            this,
            device
        ) { (gatt, connected) ->
            if (connected) {
                Log.i(TAG, "Device connected!")
                deviceConnected.value = true
                binder.connectedAction?.invoke()

                gatt.discoverServices {
                    Log.i(TAG, "Discovered device services...")
                }.withService("c4aa52a4-467e-413f-9559-419eb1a367a7") {
                    readCharacteristic("00000011-467e-413f-9559-419eb1a367a7") {
                        (chr, value) ->
                        if (chr != null && value != null) {
                            Log.i(TAG, "Received value for ${chr.uuid}: ${String(value)}")
                        }
                    }
                }
            } else {
                Log.i(TAG, "Device disconnected!")
                deviceConnected.value = false
                currentDevice.value = null
            }
        }.onSuccess {
            gatt = it
        }.onFailure {
            Log.e(TAG, "Failed to connect GATT! $it")
            stop()
        }
    }

    private fun start(intent: Intent?): Boolean {
        val action = intent?.action?.let { Json.decodeFromString<StartAction>(it) }
        if (action == null) {
            Log.e(TAG, "Received invalid intent action!")
            stop()
            return false
        }

        when(action) {
            is StartAction.Connect -> {
                val device = action.device
                Log.i(TAG, "Connecting to ${device.mac}")

                if (device == currentDevice.value) {
                    Log.i(TAG, "Already connected, skipping connection process");
                    return true
                }

                gatt?.run {
                    disconnect()
                    gatt = null
                }

                currentDevice.value = device

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
                } catch (e: IllegalArgumentException) {
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
        binder.stopAction?.invoke()
        stopSelf()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (!start(intent)) stop()
        return START_SERVICE_FLAG
    }

    private val binder = BoardServiceBinder()

    override fun onBind(intent: Intent?): IBinder {
        return binder
    }

    inner class BoardServiceBinder : Binder() {
        var connectedAction: (() -> Unit)? = null
        var stopAction: (() -> Unit)? = null
        fun getService(): BoardService = this@BoardService
    }
}