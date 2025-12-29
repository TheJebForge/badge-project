package com.thejebforge.badgeproject.gatt.command

import android.Manifest
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import androidx.annotation.RequiresPermission
import com.thejebforge.badgeproject.gatt.GATTCommandHandler
import com.thejebforge.badgeproject.gatt.GATTHelper
import com.thejebforge.badgeproject.gatt.getCharacteristic
import java.util.UUID

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

class Service internal constructor(
    val gatt: GATTHelper,
    val svcUUID: UUID
)

fun GATTHelper.withService(svcUUID: UUID, withCallback: Service.() -> Unit) = this.apply {
    withCallback(Service(this, svcUUID))
}

fun GATTHelper.withService(svcUUID: String, withCallback: Service.() -> Unit) = withService(
    UUID.fromString(svcUUID),
    withCallback
)

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