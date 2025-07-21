package zip.meows.musicopy.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.unit.dp
import musicopy.composeapp.generated.resources.Res
import musicopy.composeapp.generated.resources.chevron_forward_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.Model
import uniffi.musicopy.ServerModel
import uniffi.musicopy.TransferJobProgressModel
import zip.meows.musicopy.formatFloat
import zip.meows.musicopy.shortenNodeId
import zip.meows.musicopy.ui.screens.Transfer

@Composable
fun JobsWidget(
    model: Model,
) {
    val activeServers = model.node.servers.filter { it.accepted }

    val numJobs = activeServers.size
    val visible = numJobs > 0

    AnimatedVisibility(visible = visible) {
        Card(
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(
                modifier = Modifier.fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Text(
                    "Jobs",
                    modifier = Modifier.padding(start = 16.dp, top = 16.dp, end = 16.dp),
                    style = MaterialTheme.typography.titleLarge
                )

                Column(
                    modifier = Modifier.fillMaxWidth().padding(4.dp),
                    verticalArrangement = Arrangement.spacedBy(4.dp)
                ) {
                    for (connection in activeServers) {
                        ActiveConnectionJob(connection)
                    }

                    for (connection in activeServers) {
                        if (connection.transferJobs.any { it.progress is TransferJobProgressModel.InProgress || it.progress is TransferJobProgressModel.Failed }) {
                            ActiveTransferJob(connection)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ActiveConnectionJob(connection: ServerModel) {
    Job(
        labelLeft = {
            Text("Connected to ${connection.name}", style = MaterialTheme.typography.labelLarge)
        },
        body = {
            Column {
                Text(
                    "Node ID: ${shortenNodeId(connection.nodeId)}",
                    style = MaterialTheme.typography.bodyMedium
                )
                Text(
                    "Connection Type: ${connection.connectionType}",
                    style = MaterialTheme.typography.bodyMedium
                )
                Text(
                    "Latency: ${connection.latencyMs}ms",
                    style = MaterialTheme.typography.bodyMedium
                )
            }
        }
    )
}

@Composable
private fun ActiveTransferJob(connection: ServerModel) {
    val count = connection.transferJobs.size
    val countInProgress =
        connection.transferJobs.filter { it.progress is TransferJobProgressModel.InProgress }.size
    val countFailed =
        connection.transferJobs.filter { it.progress is TransferJobProgressModel.Failed }.size

    val countNotInProgress = count - countInProgress

    val progressPercent = countNotInProgress.toFloat() / count.toFloat()
    val progressPercentString = formatFloat(progressPercent * 100, 0)

    Job(
        labelLeft = {
            Text(
                "Transferring $count files to ${connection.name} ($progressPercentString%)",
                style = MaterialTheme.typography.labelLarge
            )
        },
        body = {
            Column {
                Text(
                    "$countInProgress remaining",
                    style = MaterialTheme.typography.bodyMedium
                )

                if (countFailed > 0) {
                    Text(
                        "$countInProgress failed",
                        style = MaterialTheme.typography.bodyMedium
                    )
                }

                for (job in connection.transferJobs) {
                    val progress = job.progress
                    if (progress is TransferJobProgressModel.Failed) {
                        Text(
                            "${job.fileRoot}/${job.filePath} failed: ${progress.error}",
                            style = MaterialTheme.typography.bodyMedium
                        )
                    }
                }
            }
        }
    )
}

@Composable
private fun Job(
    labelLeft: @Composable () -> Unit = {},
    labelRight: @Composable () -> Unit = {},
    body: (@Composable () -> Unit)? = null,
) {
    var expanded by remember { mutableStateOf(false) }
    val degrees by animateFloatAsState(if (expanded) 90f else 0f)
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainer
        )
    ) {
        Column {
            Row(
                modifier = Modifier.fillMaxWidth().clip(MaterialTheme.shapes.medium)
                    .clickable { expanded = !expanded }
            ) {
                Row(
                    modifier = Modifier.padding(8.dp, 4.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    labelLeft()

                    Box(modifier = Modifier.weight(1f))

                    labelRight()

                    Image(
                        painter = painterResource(Res.drawable.chevron_forward_24px),
                        contentDescription = "expand icon",
                        modifier = Modifier.rotate(degrees),
                        colorFilter = ColorFilter.tint(MaterialTheme.colorScheme.onSurface)
                    )
                }
            }

            AnimatedVisibility(
                visible = expanded,
            ) {
                Box(Modifier.fillMaxWidth().padding(8.dp, 4.dp)) {
                    body?.invoke()
                }
            }
        }
    }
}
