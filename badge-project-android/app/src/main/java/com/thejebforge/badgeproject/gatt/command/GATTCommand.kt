package com.thejebforge.badgeproject.gatt.command

import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import com.thejebforge.badgeproject.gatt.GATTCommandHandler

abstract class GATTCommand(
    protected val gattServer: BluetoothGatt,
    protected val commandHandler: GATTCommandHandler
) {
    abstract fun runCommand()

    open fun onServicesDiscovered(status: Int) {}

    open fun onCharacteristicRead(
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray,
        status: Int
    ) {
    }

    open fun onCharacteristicWrite(
        characteristic: BluetoothGattCharacteristic?,
        status: Int
    ) {
    }

    open fun onCharacteristicChanged(
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray
    ) {
    }

    protected fun continueExecution() {
        commandHandler.continueExecution()
    }
}