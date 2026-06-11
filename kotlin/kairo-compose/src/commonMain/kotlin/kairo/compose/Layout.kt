package kairo.compose

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box as ComposeBox
import androidx.compose.foundation.layout.Column as ComposeColumn
import androidx.compose.foundation.layout.Row as ComposeRow
import androidx.compose.foundation.layout.Spacer as ComposeSpacer
import androidx.compose.runtime.Composable

@Composable
fun Column(
    modifier: Modifier = Modifier,
    verticalArrangement: Arrangement.Vertical = Arrangement.Top,
    horizontalAlignment: HorizontalAlignment = Alignment.Start,
    content: @Composable ColumnScope.() -> Unit,
) {
    ComposeColumn(
        modifier = modifier,
        verticalArrangement = verticalArrangement,
        horizontalAlignment = horizontalAlignment,
        content = content,
    )
}

@Composable
fun Row(
    modifier: Modifier = Modifier,
    horizontalArrangement: Arrangement.Horizontal = Arrangement.Start,
    verticalAlignment: VerticalAlignment = Alignment.Top,
    content: @Composable RowScope.() -> Unit,
) {
    ComposeRow(
        modifier = modifier,
        horizontalArrangement = horizontalArrangement,
        verticalAlignment = verticalAlignment,
        content = content,
    )
}

@Composable
fun Box(
    modifier: Modifier = Modifier,
    contentAlignment: Alignment = Alignment.TopStart,
    content: @Composable BoxScope.() -> Unit,
) {
    ComposeBox(
        modifier = modifier,
        contentAlignment = contentAlignment,
        content = content,
    )
}

@Composable
fun Spacer(modifier: Modifier = Modifier) {
    ComposeSpacer(modifier = modifier)
}
