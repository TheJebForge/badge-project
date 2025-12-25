package com.thejebforge.badgeproject.gatt.command

import android.bluetooth.BluetoothGatt
import android.os.Handler
import com.thejebforge.badgeproject.gatt.GATTCommandHandler
import com.thejebforge.badgeproject.gatt.GATTHelper
import com.thejebforge.badgeproject.gatt.getCharacteristic
import com.thejebforge.badgeproject.util.BoardConstants

class EnableCommands(
    gattServer: BluetoothGatt,
    commandHandler: GATTCommandHandler,
    private val handler: Handler,
    private val onDone: (Boolean) -> Unit
) : GATTCommand(gattServer, commandHandler) {
    override fun runCommand() {
        val respChr = gattServer.getCharacteristic(BoardConstants.CHARACTER_SVC, BoardConstants.RESPONSE_CHR)

        if (respChr == null) {
            onDone(false)
            continueExecution()
            return
        }

        if (!gattServer.setCharacteristicNotification(respChr, true)) {
            onDone(false)
            continueExecution()
            return
        }

        handler.postDelayed({
            onDone(true)
            continueExecution()
        }, 500)
    }
}

fun GATTHelper.enableCommands(callback: (Boolean) -> Unit) = this.apply {
    commandHandler.appendCommand(EnableCommands(
        gattServer,
        commandHandler,
        handler,
        callback
    ))
}