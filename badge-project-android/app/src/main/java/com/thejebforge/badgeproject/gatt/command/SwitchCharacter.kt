package com.thejebforge.badgeproject.gatt.command

import android.bluetooth.BluetoothGatt
import android.os.Handler
import com.thejebforge.badgeproject.gatt.GATTCommandHandler
import com.thejebforge.badgeproject.gatt.GATTHelper
import com.thejebforge.badgeproject.util.PayloadCreator

class SwitchCharacter(
    gattServer: BluetoothGatt,
    commandHandler: GATTCommandHandler,
    handler: Handler,
    private val name: String,
    private val onDone: (Boolean) -> Unit
) : ActionCommand(gattServer, commandHandler, handler) {
    override fun payload(): ByteArray = PayloadCreator.switchCharacter(name)

    override fun received(data: ByteArray) {
        onDone(true)
    }

    override fun failed(reason: ActionResponse<Nothing>) {
        onDone(false)
    }
}

fun GATTHelper.switchCharacter(name: String, callback: (Boolean) -> Unit) = this.apply {
    commandHandler.appendCommand(SwitchCharacter(
        gattServer,
        commandHandler,
        handler,
        name,
        callback
    ))
}