package com.thejebforge.badgeproject.gatt

import android.Manifest
import android.annotation.SuppressLint
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothProfile
import android.content.Context
import android.content.pm.PackageManager
import android.os.Handler
import androidx.core.app.ActivityCompat
import java.util.UUID

@SuppressLint("MissingPermission")
class GATTHelper private constructor(
    private val connectionCallback: (Pair<GATTHelper, Boolean>) -> Unit,
    val handler: Handler
) : BluetoothGattCallback() {
    //region Connection to the device
    lateinit var gattServer: BluetoothGatt
    val commandHandler = GATTCommandHandler()

    companion object {
        fun connect(
            context: Context,
            handler: Handler,
            device: BluetoothDevice,
            connectionCallback: (Pair<GATTHelper, Boolean>) -> Unit
        ): Result<GATTHelper> = runCatching {
            GATTHelper(
                connectionCallback,
                handler
            ).apply {
                if (ActivityCompat.checkSelfPermission(
                        context,
                        Manifest.permission.BLUETOOTH_CONNECT
                    ) != PackageManager.PERMISSION_GRANTED
                ) {
                    throw IllegalStateException("No bluetooth permission")
                }
                gattServer = device.connectGatt(context, false, this)
            }
        }
    }

    //endregion

    //region Public API

    fun disconnect() = gattServer.disconnect()
    //endregion

    //region Overrides
    override fun onConnectionStateChange(
        gatt: BluetoothGatt?,
        status: Int,
        newState: Int
    ) {
        when (newState) {
            BluetoothProfile.STATE_CONNECTED -> {
                connectionCallback(Pair(this, true))
            }

            BluetoothProfile.STATE_DISCONNECTED -> {
                connectionCallback(Pair(this, false))
            }
        }
    }

    override fun onServicesDiscovered(gatt: BluetoothGatt?, status: Int) {
        super.onServicesDiscovered(gatt, status)

        commandHandler.currentCommand?.onServicesDiscovered(status)
    }

    override fun onCharacteristicRead(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray,
        status: Int
    ) {
        super.onCharacteristicRead(gatt, characteristic, value, status)

        commandHandler.currentCommand?.onCharacteristicRead(characteristic, value, status)
    }

    override fun onCharacteristicWrite(
        gatt: BluetoothGatt?,
        characteristic: BluetoothGattCharacteristic?,
        status: Int
    ) {
        super.onCharacteristicWrite(gatt, characteristic, status)

        commandHandler.currentCommand?.onCharacteristicWrite(characteristic, status)
    }

    override fun onCharacteristicChanged(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray
    ) {
        super.onCharacteristicChanged(gatt, characteristic, value)

        commandHandler.currentCommand?.onCharacteristicChanged(characteristic, value)
    }

    //endregion
}

fun BluetoothGatt.getCharacteristic(svcUUID: UUID, chrUUID: UUID): BluetoothGattCharacteristic? =
    getService(svcUUID)
        ?.getCharacteristic(chrUUID)

fun BluetoothGatt.getCharacteristic(svcUUID: String, chrUUID: String): BluetoothGattCharacteristic? =
    getService(UUID.fromString(svcUUID))
        ?.getCharacteristic(UUID.fromString(chrUUID))