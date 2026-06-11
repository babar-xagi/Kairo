package kairo.compose

import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.ui.unit.dp

val Int.dpValue: Dp
    get() = dp

val Float.dpValue: Dp
    get() = dp

fun Modifier.pad(all: Dp): Modifier = padding(all)

fun Modifier.pad(all: Int): Modifier = padding(all.dp)

fun Modifier.fill(): Modifier = fillMaxSize()

fun Modifier.fillWidth(fraction: Float = 1f): Modifier = fillMaxWidth(fraction)

fun Modifier.fillHeight(fraction: Float = 1f): Modifier = fillMaxHeight(fraction)

fun Modifier.widthDp(value: Int): Modifier = width(value.dp)

fun Modifier.heightDp(value: Int): Modifier = height(value.dp)

fun Modifier.sizeDp(value: Int): Modifier = size(value.dp)

