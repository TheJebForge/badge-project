package com.thejebforge.badgeproject.gatt.command

import com.thejebforge.badgeproject.gatt.GATTHelper
import java.util.UUID

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