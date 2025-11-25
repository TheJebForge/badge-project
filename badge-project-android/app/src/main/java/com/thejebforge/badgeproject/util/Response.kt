package com.thejebforge.badgeproject.util

sealed class Response<out T> {
    data class Success<out T>(val data: T) : Response<T>()
    data object Loading : Response<Nothing>()
    data class Error(val exception: Throwable) : Response<Nothing>()
}