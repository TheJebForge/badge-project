package com.thejebforge.badgeproject.data.entity

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity
data class PreviousDevice (
    @PrimaryKey
    val macAddress: String,
    val name: String
)
