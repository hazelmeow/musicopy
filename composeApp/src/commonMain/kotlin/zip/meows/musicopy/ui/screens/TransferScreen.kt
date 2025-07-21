package zip.meows.musicopy.ui.screens

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ProgressIndicatorDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import musicopy.composeapp.generated.resources.Res
import musicopy.composeapp.generated.resources.chevron_forward_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.ClientModel
import uniffi.musicopy.TransferJobModel
import uniffi.musicopy.TransferJobProgressModel
import zip.meows.musicopy.formatFloat
import zip.meows.musicopy.ui.components.SectionCard

@Composable
fun TransferScreen(
    clientModel: ClientModel,
    onCancel: () -> Unit,
) {
    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
    ) {
        SectionCard(
            title = "Transferring ${clientModel.transferJobs.size} files",
            body = {
                LinearProgressIndicator(
                    progress = {
                        val count = clientModel.transferJobs.size
                        val countInProgress =
                            clientModel.transferJobs.filter { it.progress is TransferJobProgressModel.InProgress }.size
                        val countNotInProgress = count - countInProgress

                        return@LinearProgressIndicator if (count == 0) {
                            0f
                        } else {
                            countNotInProgress.toFloat() / count.toFloat()
                        }
                    },
                    modifier = Modifier.fillMaxWidth()
                )

                Text("lorem")
            },
            onCancel = onCancel,
        )

        val jobs = clientModel.transferJobs.sortedBy { it.startedAt }.reversed()
        for (job in jobs) {
            Card(
                modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp)
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(8.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            formatJobName(job),
                            style = MaterialTheme.typography.labelLarge,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis
                        )
                        Text(
                            formatJobSubtitle(job),
                            style = MaterialTheme.typography.labelMedium
                        )
                    }

                    Box(
                        modifier = Modifier.size(30.dp),
                        contentAlignment = Alignment.Center
                    ) {
                        val progress = job.progress
                        when (progress) {
                            is TransferJobProgressModel.InProgress -> {
                                var targetProgress by remember { mutableFloatStateOf(0f) }
                                val animatedProgress by animateFloatAsState(
                                    targetValue = targetProgress,
                                    animationSpec = ProgressIndicatorDefaults.ProgressAnimationSpec,
                                )

                                LaunchedEffect(true) {
                                    while (isActive) {
                                        targetProgress =
                                            progress.bytes.get().toFloat() / job.fileSize.toFloat()
                                        delay(100)
                                    }
                                }

                                CircularProgressIndicator(
                                    progress = {
                                        animatedProgress
                                    },
                                )
                            }

                            is TransferJobProgressModel.Finished -> {
                                Icon(
                                    painter = painterResource(Res.drawable.chevron_forward_24px),
                                    contentDescription = null,
                                )
                            }

                            is TransferJobProgressModel.Failed -> {
                                Icon(
                                    painter = painterResource(Res.drawable.chevron_forward_24px),
                                    contentDescription = null,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

internal fun formatJobName(job: TransferJobModel): String {
    val pathParts = job.filePath.split("/")
    return "${job.fileRoot}/.../${pathParts.last()}"
}

internal fun formatJobSubtitle(job: TransferJobModel): String {
    val sizeMB = job.fileSize.toFloat() / 1_000_000f
    val sizeMBString = formatFloat(sizeMB, 2)

    val progress = job.progress
    return when (progress) {
        is TransferJobProgressModel.Finished -> "$sizeMBString MB"
        is TransferJobProgressModel.InProgress -> ""
        is TransferJobProgressModel.Failed -> "Error: ${progress.error}"
    }
}