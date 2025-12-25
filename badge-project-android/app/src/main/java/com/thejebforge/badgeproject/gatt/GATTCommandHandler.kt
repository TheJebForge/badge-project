package com.thejebforge.badgeproject.gatt

import com.thejebforge.badgeproject.gatt.command.GATTCommand

class GATTCommandHandler {
    private val commandDeque: ArrayDeque<GATTCommand> = ArrayDeque<_>()
    var currentCommand: GATTCommand? = null
        private set
    private var waiting: Boolean = false

    fun appendCommand(command: GATTCommand) {
        commandDeque.add(command)
        tryStartExecution()
    }

    fun tryStartExecution() {
        if (currentCommand != null && waiting) return
        if (commandDeque.isEmpty()) return

        waiting = true
        val command = commandDeque.removeFirst()
        currentCommand = command

        currentCommand?.runCommand()
    }

    fun continueExecution() {
        waiting = false
        tryStartExecution()
    }
}