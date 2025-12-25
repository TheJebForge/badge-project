package com.thejebforge.badgeproject.util

fun <E, R> callbackCollector(
    iterable: Iterable<E>,
    onEach: (E, (R) -> Unit) -> Unit,
    onCollected: (List<R>) -> Unit
) {
    val elements = iterable.toList()
    val results = MutableList<PotentiallyMissingValue<R>>(elements.size) { PotentiallyMissingValue.Missing }

    val checkDone: () -> Unit = {
        if (!results.contains(PotentiallyMissingValue.Missing)) {
            onCollected(results.map {
                (it as PotentiallyMissingValue.Existing<R>).value
            })
        }
    }

    elements.forEachIndexed { index, element ->
        onEach(element) { result ->
            results[index] = PotentiallyMissingValue.Existing(result)
            checkDone()
        }
    }
}

internal sealed class PotentiallyMissingValue<out T> {
    data class Existing<T>(
        val value: T
    ) : PotentiallyMissingValue<T>()

    object Missing : PotentiallyMissingValue<Nothing>()
}