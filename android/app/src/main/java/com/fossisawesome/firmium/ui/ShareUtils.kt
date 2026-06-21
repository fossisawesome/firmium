package com.fossisawesome.firmium.ui

import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import androidx.core.content.FileProvider
import java.io.File
import java.io.FileOutputStream

// Shares stats exports and recap images via the system share sheet, using the
// FileProvider already declared in AndroidManifest.xml (cache-path in file_paths.xml).
object ShareUtils {

    fun shareText(context: Context, fileName: String, text: String, mime: String) {
        val file = File(context.cacheDir, fileName)
        file.writeText(text)
        shareFile(context, file, mime)
    }

    fun shareBitmap(context: Context, fileName: String, bitmap: Bitmap) {
        val file = File(context.cacheDir, fileName)
        FileOutputStream(file).use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
        shareFile(context, file, "image/png")
    }

    private fun shareFile(context: Context, file: File, mime: String) {
        val uri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", file)
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = mime
            putExtra(Intent.EXTRA_STREAM, uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        context.startActivity(
            Intent.createChooser(intent, "Share").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        )
    }
}
