package com.thejebforge.badgeproject.gatt.command

import android.Manifest
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import androidx.annotation.RequiresPermission
import com.thejebforge.badgeproject.gatt.GATTCommandHandler
import com.thejebforge.badgeproject.gatt.GATTHelper
import com.thejebforge.badgeproject.gatt.getCharacteristic
import com.thejebforge.badgeproject.gatt.toBoolean
import com.thejebforge.badgeproject.gatt.toInt
import com.thejebforge.badgeproject.gatt.toStringWithoutNulls
import java.util.*

class ReadCharacteristic(
    gattServer: BluetoothGatt,
    commandHandler: GATTCommandHandler,
    private val svcUUID: UUID,
    private val chrUUID: UUID,
    private val onRead: (Pair<BluetoothGattCharacteristic?, ByteArray?>) -> Unit
) : GATTCommand(gattServer, commandHandler) {
    @RequiresPermission(Manifest.permission.BLUETOOTH_CONNECT)
    override fun runCommand() {
        val chr = gattServer.getCharacteristic(svcUUID, chrUUID)

        if (!gattServer.readCharacteristic(chr)) {
            onRead(Pair(null, null))
            continueExecution()
        }
    }

    override fun onCharacteristicRead(characteristic: BluetoothGattCharacteristic, value: ByteArray, status: Int) {
        onRead(
            Pair(
                characteristic,
                if (status == BluetoothGatt.GATT_SUCCESS) value else null
            )
        )
        continueExecution()
    }
}

fun Service.readCharacteristic(chrUUID: UUID, readCallback: (Pair<BluetoothGattCharacteristic?, ByteArray?>) -> Unit) {
    with(gatt) {
        commandHandler.appendCommand(
            ReadCharacteristic(
                gattServer,
                commandHandler,
                svcUUID,
                chrUUID,
                readCallback
            )
        )
    }
}

fun Service.readCharacteristic(chrUUID: String, readCallback: (Pair<BluetoothGattCharacteristic?, ByteArray?>) -> Unit) {
    readCharacteristic(UUID.fromString(chrUUID), readCallback)
}

fun GATTHelper.readCharacteristicToString(svc: String, chr: String, callback: (String?) -> Unit) = this.apply {
    withService(svc) {
        readCharacteristic(chr) {
                (chr, data) ->
            if (chr == null || data == null) {
                callback(null)
                return@readCharacteristic
            }

            callback(data.toStringWithoutNulls())
        }
    }
}

fun GATTHelper.readCharacteristicToInt(svc: String, chr: String, callback: (Int?) -> Unit) = this.apply {
    withService(svc) {
        readCharacteristic(chr) {
                (chr, data) ->
            if (chr == null || data == null) {
                callback(null)
                return@readCharacteristic
            }

            callback(data.toInt())
        }
    }
}

fun GATTHelper.readCharacteristicToBool(svc: String, chr: String, callback: (Boolean?) -> Unit) = this.apply {
    withService(svc) {
        readCharacteristic(chr) {
                (chr, data) ->
            if (chr == null || data == null) {
                callback(null)
                return@readCharacteristic
            }

            callback(data.toBoolean())
        }
    }
}