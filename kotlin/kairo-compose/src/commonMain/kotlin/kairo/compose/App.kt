package kairo.compose

import androidx.compose.runtime.Composable

data class KairoAppDefinition internal constructor(
    val title: String,
    val content: @Composable () -> Unit,
)

private var registeredApp: KairoAppDefinition? = null

fun kairoApp(title: String, content: @Composable () -> Unit) {
    registeredApp = KairoAppDefinition(title = title, content = content)
}

fun currentKairoApp(): KairoAppDefinition? = registeredApp

