package zip.meows.musicopy.ui

import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.painter.Painter
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.core.DragIndication
import com.composables.core.ModalBottomSheet
import com.composables.core.ModalBottomSheetState
import com.composables.core.Scrim
import com.composables.core.Sheet
import com.composables.core.SheetDetent
import com.composables.core.SheetDetent.Companion.FullyExpanded
import com.composables.core.SheetDetent.Companion.Hidden
import com.composables.core.rememberModalBottomSheetState
import kotlinx.coroutines.runBlocking
import musicopy.composeapp.generated.resources.Res
import musicopy.composeapp.generated.resources.arrow_downward_24px
import musicopy.composeapp.generated.resources.arrow_upward_24px
import musicopy.composeapp.generated.resources.cell_tower_24px
import musicopy.composeapp.generated.resources.connections_label
import musicopy.composeapp.generated.resources.content_copy_24px
import musicopy.composeapp.generated.resources.copy_button_description
import musicopy.composeapp.generated.resources.home_relay_label
import musicopy.composeapp.generated.resources.network_node_24px
import musicopy.composeapp.generated.resources.node_id_label
import musicopy.composeapp.generated.resources.p2p_24px
import musicopy.composeapp.generated.resources.received_label
import musicopy.composeapp.generated.resources.sent_label
import org.jetbrains.compose.resources.painterResource
import org.jetbrains.compose.resources.stringResource
import uniffi.musicopy.Model
import zip.meows.musicopy.toClipEntry


val Peek = SheetDetent(identifier = "peek") { containerHeight, sheetHeight ->
    containerHeight * 0.6f
}

class NodeStatusSheetState(
    internal val inner: ModalBottomSheetState,
) {
//    var targetDetent: SheetDetent
//        get() {
//            return inner.targetDetent
//        }
//        set(value) {
//            inner.targetDetent = value
//        }

    fun peek() {
        inner.targetDetent = Peek
    }
}

@Composable
fun rememberNodeStatusSheetState(): NodeStatusSheetState {
    val inner = rememberModalBottomSheetState(
        initialDetent = Hidden,
        detents = listOf(Hidden, Peek, FullyExpanded)
    )
    return NodeStatusSheetState(
        inner,
    )
}

@Composable
fun NodeStatusSheet(state: NodeStatusSheetState, model: Model? = null) {
    ModalBottomSheet(state = state.inner) {
        Scrim(
            enter = fadeIn(),
            exit = fadeOut()
        )

        Sheet(
            modifier = Modifier
                .shadow(4.dp, RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp))
                .clip(RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp))
                .background(Color.White)
                .widthIn(max = 640.dp)
                .fillMaxWidth()
                .imePadding()
        ) {
            Column {
                Box(
                    modifier = Modifier.fillMaxWidth(),
                    contentAlignment = Alignment.TopCenter
                ) {
                    DragIndication(
                        modifier = Modifier
                            .padding(top = 8.dp)
                            .background(Color.Black.copy(0.4f), RoundedCornerShape(100))
                            .width(32.dp)
                            .height(4.dp)
                    )
                }

                model?.let {
                    Column(
                        modifier = Modifier.padding(8.dp).padding(bottom = 20.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        model.node?.let { node ->
                            StatusDetail(
                                label = stringResource(resource = Res.string.node_id_label),
                                value = "${node.nodeId.slice(0..<6)}...${node.nodeId.slice((node.nodeId.length - 6)..<(node.nodeId.length))}",
                                iconPainter = painterResource(Res.drawable.network_node_24px),
                                textToCopy = node.nodeId
                            )

                            StatusDetail(
                                label = stringResource(resource = Res.string.home_relay_label),
                                value = node.homeRelay,
                                iconPainter = painterResource(Res.drawable.cell_tower_24px),
                                textToCopy = node.homeRelay
                            )

                            StatusDetail(
                                label = stringResource(resource = Res.string.connections_label),
                                value = "${node.connSuccess} success, ${node.connDirect} direct",
                                iconPainter = painterResource(Res.drawable.p2p_24px),
                            )

                            StatusDetail(
                                label = stringResource(resource = Res.string.sent_label),
                                value = "${node.sendIpv4} v4, ${node.sendIpv6} v6, ${node.sendRelay} relay",
                                iconPainter = painterResource(Res.drawable.arrow_upward_24px),
                            )

                            StatusDetail(
                                label = stringResource(resource = Res.string.received_label),
                                value = "${node.recvIpv4} v4, ${node.recvIpv6} v6, ${node.recvRelay} relay",
                                iconPainter = painterResource(Res.drawable.arrow_downward_24px),
                            )
                        }
                    }
                } ?: run {
                    CircularProgressIndicator()
                }
            }
        }
    }
}

@Composable
private fun StatusDetail(
    label: String,
    value: String,
    iconPainter: Painter,
    textToCopy: String? = null,
) {
    Card(
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainer
        ),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier
                .fillMaxWidth()
                .height(50.dp)
        ) {
            Icon(
                painter = iconPainter,
                contentDescription = label,
                modifier = Modifier.padding(8.dp)
            )

            Column(modifier = Modifier.weight(1f)) {
                Text(label, style = MaterialTheme.typography.labelLarge)
                Text(
                    value,
                    style = MaterialTheme.typography.bodyLarge,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }

            textToCopy?.let { textToCopy ->
                CopyIconButton(
                    textToCopy = textToCopy,
                    contentDescription = stringResource(resource = Res.string.copy_button_description)
                )
            }
        }
    }
}

@Composable
fun CopyIconButton(textToCopy: String, contentDescription: String) {
    val clipboard = LocalClipboard.current

    IconButton(
        onClick = {
            runBlocking {
                val clip = toClipEntry(textToCopy)
                clipboard.setClipEntry(clip)
                // not supported in CMP
                // Toast.makeText(context, "Copied to clipboard", Toast.LENGTH_SHORT).show()
            }
        },
    ) {
        Icon(
            painter = painterResource(Res.drawable.content_copy_24px),
            contentDescription = contentDescription
        )
    }
}
