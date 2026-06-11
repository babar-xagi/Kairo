package kairo.compose

import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.State
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.snapshots.SnapshotStateList
import kotlin.reflect.KProperty

@Composable
fun <T> state(initial: T): MutableState<T> = remember { mutableStateOf(initial) }

@Composable
fun <T> stateOf(initial: T): MutableState<T> = state(initial)

@Composable
fun <T> listState(vararg initialItems: T): SnapshotStateList<T> =
    remember { mutableStateListOf(*initialItems) }

operator fun <T> State<T>.getValue(thisObj: Any?, property: KProperty<*>): T = value

operator fun <T> MutableState<T>.setValue(
    thisObj: Any?,
    property: KProperty<*>,
    newValue: T,
) {
    value = newValue
}

