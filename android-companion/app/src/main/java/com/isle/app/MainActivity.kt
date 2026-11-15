package com.isle.app

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.provider.OpenableColumns
import android.provider.Settings
import android.text.InputType
import android.view.Gravity
import android.view.ViewGroup
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import org.json.JSONObject
import java.io.BufferedInputStream
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.Executors

class MainActivity : Activity() {
    private val executor = Executors.newSingleThreadExecutor()
    private lateinit var hostInput: EditText
    private lateinit var codeInput: EditText
    private lateinit var status: TextView
    private lateinit var pairButton: Button
    private lateinit var fileButton: Button
    private val prefs by lazy { getSharedPreferences("isle", MODE_PRIVATE) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val column = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(24), dp(28), dp(24), dp(24))
            gravity = Gravity.START
        }

        column.addView(title("Isle Companion"))
        column.addView(body("Mirror Android notifications into the Isle island on your PC and send files to it. Both devices must be on the same Wi-Fi network."))

        column.addView(label("Isle address (PC)"))
        hostInput = EditText(this).apply {
            hint = "192.168.1.10:45872"
            inputType = InputType.TYPE_CLASS_TEXT
            setText(prefs.getString("host", ""))
            setSingleLine()
        }
        column.addView(hostInput)

        column.addView(label("Pairing code"))
        codeInput = EditText(this).apply {
            hint = "6-digit code from Isle"
            inputType = InputType.TYPE_CLASS_NUMBER
            setSingleLine()
        }
        column.addView(codeInput)

        pairButton = Button(this).apply {
            text = "Pair"
            setOnClickListener { pair() }
        }
        column.addView(pairButton, fullWidth())

        status = body("Not paired yet.")
        column.addView(status)

        column.addView(label("Notifications"))
        column.addView(body("Allow notification access, then notifications will be forwarded automatically while Isle is running."))
        column.addView(Button(this).apply {
            text = "Open notification access settings"
            setOnClickListener {
                startActivity(Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS))
            }
        }, fullWidth())

        column.addView(label("Files"))
        column.addView(body("Pick any file. It is uploaded to Downloads\\Isle Phone on the PC."))
        fileButton = Button(this).apply {
            text = "Send a file"
            isEnabled = prefs.getString("token", null) != null
            setOnClickListener { pickFile() }
        }
        column.addView(fileButton, fullWidth())

        column.addView(Button(this).apply {
            text = "Unpair"
            setOnClickListener {
                prefs.edit().remove("token").apply()
                status.text = "Unpaired."
                fileButton.isEnabled = false
            }
        }, fullWidth())

        if (prefs.getString("token", null) != null) {
            status.text = "Paired with ${prefs.getString("host", "Isle")}."
        }

        val scroll = ScrollView(this).apply { addView(column) }
        setContentView(scroll)
    }

    private fun pair() {
        val address = hostInput.text.toString().trim()
        val code = codeInput.text.toString().trim()
        if (address.isEmpty() || code.length != 6) {
            status.text = "Enter the PC address and the 6-digit code."
            return
        }
        pairButton.isEnabled = false
        status.text = "Pairing…"
        executor.execute {
            try {
                val url = URL("http://$address/pair")
                val connection = (url.openConnection() as HttpURLConnection).apply {
                    requestMethod = "POST"
                    connectTimeout = 6000
                    readTimeout = 10000
                    doOutput = true
                    setRequestProperty("Content-Type", "application/json")
                }
                val payload = JSONObject()
                    .put("code", code)
                    .put("device", "${android.os.Build.MANUFACTURER} ${android.os.Build.MODEL}")
                    .toString()
                connection.outputStream.use { it.write(payload.toByteArray()) }
                val statusCode = connection.responseCode
                val text = if (statusCode in 200..299) {
                    connection.inputStream.bufferedReader().use(BufferedReader::readText)
                } else {
                    connection.errorStream?.bufferedReader()?.use(BufferedReader::readText) ?: ""
                }
                if (statusCode in 200..299) {
                    val token = JSONObject(text).getString("token")
                    prefs.edit().putString("host", address).putString("token", token).apply()
                    runOnUiThread {
                        status.text = "Paired. Notifications and files are ready."
                        fileButton.isEnabled = true
                        codeInput.setText("")
                    }
                } else {
                    runOnUiThread { status.text = "Pairing failed: ${text.take(160)}" }
                }
            } catch (error: Exception) {
                runOnUiThread { status.text = "Pairing failed: ${error.message}" }
            } finally {
                runOnUiThread { pairButton.isEnabled = true }
            }
        }
    }

    private fun pickFile() {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "*/*"
        }
        @Suppress("DEPRECATION")
        startActivityForResult(intent, REQUEST_FILE)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != REQUEST_FILE || resultCode != RESULT_OK) return
        val uri = data?.data ?: return
        val name = displayName(uri)
        status.text = "Uploading $name…"
        executor.execute {
            try {
                val host = prefs.getString("host", null) ?: return@execute
                val token = prefs.getString("token", null) ?: return@execute
                val encoded = java.net.URLEncoder.encode(name, "UTF-8")
                val url = URL("http://$host/file?name=$encoded")
                val connection = (url.openConnection() as HttpURLConnection).apply {
                    requestMethod = "POST"
                    connectTimeout = 6000
                    readTimeout = 120000
                    doOutput = true
                    setRequestProperty("X-Isle-Token", token)
                    setRequestProperty("Content-Type", "application/octet-stream")
                }
                contentResolver.openInputStream(uri)?.use { input ->
                    connection.outputStream.use { output ->
                        input.copyTo(output)
                    }
                }
                val statusCode = connection.responseCode
                val text = connection.inputStream.bufferedReader().use(BufferedReader::readText)
                runOnUiThread {
                    status.text = if (statusCode in 200..299) {
                        "Sent $name."
                    } else {
                        "Upload failed: ${text.take(160)}"
                    }
                }
            } catch (error: Exception) {
                runOnUiThread { status.text = "Upload failed: ${error.message}" }
            }
        }
    }

    private fun displayName(uri: Uri): String {
        contentResolver.query(uri, null, null, null, null)?.use { cursor ->
            val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (index >= 0 && cursor.moveToFirst()) {
                return cursor.getString(index) ?: "file"
            }
        }
        return "file"
    }

    override fun onDestroy() {
        executor.shutdown()
        super.onDestroy()
    }

    private fun title(value: String) = TextView(this).apply {
        text = value
        textSize = 24f
        setPadding(0, 0, 0, dp(8))
    }

    private fun label(value: String) = TextView(this).apply {
        text = value
        textSize = 13f
        setPadding(0, dp(16), 0, dp(4))
    }

    private fun body(value: String) = TextView(this).apply {
        text = value
        textSize = 14f
        setPadding(0, dp(6), 0, dp(6))
    }

    private fun fullWidth() = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.WRAP_CONTENT,
    )

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    companion object {
        private const val REQUEST_FILE = 41
    }
}
