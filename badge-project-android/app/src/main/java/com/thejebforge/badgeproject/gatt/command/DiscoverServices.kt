package com.thejebforge.badgeproject.gatt.command

import android.bluetooth.BluetoothGatt
import com.thejebforge.badgeproject.gatt.GATTCommandHandler
import com.thejebforge.badgeproject.gatt.GATTHelper

class DiscoverServices(
    gattServer: BluetoothGatt,
    commandHandler: GATTCommandHandler,
    private val onDiscovered: (success: Boolean) -> Unit
) : GATTCommand(
    gattServer,
    commandHandler
) {
    override fun runCommand() {
        if (!gattServer.discoverServices()) {
            onDiscovered(false)
            continueExecution()
        }
    }

    override fun onServicesDiscovered(status: Int) {
        onDiscovered(status == BluetoothGatt.GATT_SUCCESS)
        continueExecution()
    }
}

fun GATTHelper.discoverServices(callback: (Boolean) -> Unit) = this.apply {
    commandHandler.appendCommand(DiscoverServices(
        gattServer,
        commandHandler,
        callback
    ))
}