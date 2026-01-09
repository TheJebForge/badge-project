package com.thejebforge.badgeproject.gatt.command

import android.Manifest
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothStatusCodes
import androidx.annotation.RequiresPermission
import com.thejebforge.badgeproject.gatt.GATTCommandHandler
import com.thejebforge.badgeproject.gatt.GATTHelper
import com.thejebforge.badgeproject.gatt.getCharacteristic
import kotlinx.serialization.Serializable
import java.nio.ByteBuffer
import java.util.UUID

class WriteCharacteristic(
    gattServer: BluetoothGatt,
    commandHandler: GATTCommandHandler,
    private val svcUUID: UUID,
    private val chrUUID: UUID,
    private val data: ByteArray,
    private val writeType: Int,
    private val onWritten: (BluetoothGattCharacteristic?) -> Unit
) : GATTCommand(gattServer, commandHandler) {
    @RequiresPermission(Manifest.permission.BLUETOOTH_CONNECT)
    override fun runCommand() {
        val chr = gattServer.getCharacteristic(svcUUID, chrUUID)

        if (chr == null
            || gattServer.writeCharacteristic(chr, data, writeType) != BluetoothStatusCodes.SUCCESS
        ) {
            onWritten(null)
            continueExecution()
        }
    }

    override fun onCharacteristicWrite(
        characteristic: BluetoothGattCharacteristic?,
        status: Int
    ) {
        onWritten(
            characteristic
        )
        continueExecution()
    }
}

fun Service.writeCharacteristic(chrUUID: UUID, data: ByteArray, writeType: Int, callback: (BluetoothGattCharacteristic?) -> Unit) {
    with(gatt) {
        commandHandler.appendCommand(
            WriteCharacteristic(
                gattServer,
                commandHandler,
                svcUUID,
                chrUUID,
                data,
                writeType,
                callback
            )
        )
    }
}

fun Service.writeCharacteristic(chrUUID: String, data: ByteArray, writeType: Int, callback: (BluetoothGattCharacteristic?) -> Unit) {
    writeCharacteristic(UUID.fromString(chrUUID), data, writeType, callback)
}

fun Service.writeCharacteristic(chrUUID: UUID, data: ByteArray, callback: (BluetoothGattCharacteristic?) -> Unit) {
    writeCharacteristic(chrUUID, data, BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT, callback)
}

fun Service.writeCharacteristic(chrUUID: String, data: ByteArray, callback: (BluetoothGattCharacteristic?) -> Unit) {
    writeCharacteristic(UUID.fromString(chrUUID), data, callback)
}

fun Boolean.toByteArray(): ByteArray {
    val buffer = ByteBuffer.allocate(1)
    buffer.put(if (this) 1.toByte() else 0.toByte())
    return buffer.array()
}

fun GATTHelper.writeCharacteristicBoolean(svc: String, chr: String, value: Boolean, callback: (Boolean) -> Unit) = this.apply {
    withService(svc) {
        writeCharacteristic(chr, value.toByteArray()) {
            callback(it != null)
        }
    }
}