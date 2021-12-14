// Copyright (c) 2021 Open Community Project Association https://ocpa.ch
// This software is published under the AGPLv3 license.

package net.qaul.ble.service

import android.bluetooth.*
import android.bluetooth.le.*
import android.content.Context
import android.content.Intent
import android.os.Handler
import android.os.Looper
import android.os.ParcelUuid
import androidx.lifecycle.LifecycleService
import net.qaul.ble.AppLog
import net.qaul.ble.RemoteLog
import net.qaul.ble.core.BaseBleActor
import net.qaul.ble.model.BLEScanDevice
import java.util.*

class BleService : LifecycleService() {
    private var bleCallback: BleScanCallBack? = null
    private val TAG: String = BleService::class.java.simpleName
    private var bluetoothAdapter: BluetoothAdapter? = null
    private var bleAdvertiseCallback: BleAdvertiseCallback? = null
    private var qaulId: ByteArray? = null
    private var advertMode = ""
    private var bluetoothLeAdvertiser: BluetoothLeAdvertiser? = null
    private var gattServer: BluetoothGattServer? = null
    private var bluetoothManager: BluetoothManager? = null

    private lateinit var scanCallback: ScanCallback
    private lateinit var bleScanner: BluetoothLeScanner
    private val outOfRangeChecker = Handler(Looper.getMainLooper())
    private val devicesList = arrayListOf<BLEScanDevice>()
    private val ignoreList = arrayListOf<BLEScanDevice>()
    private val blackList = arrayListOf<BLEScanDevice>()
    private val uuidList = arrayListOf<ParcelUuid>()
    private var filters: ArrayList<ScanFilter> = arrayListOf()
    private var scanSettings: ScanSettings? = null

    private val handler = Handler(Looper.getMainLooper())

    companion object {
        var bleService: BleService? = null
        var isAdvertisementRunning = false
        var isScanningRunning = false
        public val SERVICE_UUID = "99E91399-80ED-4943-9BCB-39C532A76023"
        public val READ_CHAR = "99E91401-80ED-4943-9BCB-39C532A76023"
    }

    override fun onCreate() {
        super.onCreate()
        bleService = this
        AppLog.e(TAG, "$TAG created")
    }

    override fun onStart(intent: Intent?, startId: Int) {
        super.onStart(intent, startId)
        AppLog.e(TAG, "$TAG started")
    }

    /**
     * This Method Will Set the necessary data and start the advertisement
     */
    fun startAdvertise(
        qaul_id: ByteArray,
        mode: String, bleCallback: BleAdvertiseCallback
    ) {
        bleService?.qaulId = qaul_id
        bleService?.advertMode = mode
        bleService?.bleAdvertiseCallback = bleCallback

        val t = Thread {
            bluetoothManager = bleService!!.getSystemService(BLUETOOTH_SERVICE) as BluetoothManager
            bluetoothAdapter = bluetoothManager!!.adapter
            bluetoothAdapter!!.name = "qaul"
            bluetoothLeAdvertiser = bluetoothAdapter!!.bluetoothLeAdvertiser
            if (bluetoothAdapter != null) {
                AppLog.e(
                    TAG,
                    "Peripheral supported"
                )
                val firstService = BluetoothGattService(
                    UUID.fromString(SERVICE_UUID),
                    BluetoothGattService.SERVICE_TYPE_PRIMARY
                )
                val firstServiceChar = BluetoothGattCharacteristic(
                    UUID.fromString(READ_CHAR),
                    BluetoothGattCharacteristic.PROPERTY_READ,
                    BluetoothGattCharacteristic.PERMISSION_READ
                )

                firstServiceChar.value = qaulId
                firstService.addCharacteristic(firstServiceChar)

                startGattServer(service = firstService)

                val dataBuilder = AdvertiseData.Builder()
                val settingsBuilder = AdvertiseSettings.Builder()
                dataBuilder.setIncludeTxPowerLevel(true)
                val uuid = ParcelUuid(UUID.fromString(SERVICE_UUID))
                dataBuilder.addServiceUuid(uuid)
                dataBuilder.setIncludeDeviceName(true)
                when (advertMode) {
                    "low_power" -> {
                        settingsBuilder
                            .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_LOW_POWER)
                    }
                    "balanced" -> {
                        settingsBuilder
                            .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_BALANCED)
                    }
                    "low_latency" -> {
                        settingsBuilder
                            .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_LOW_LATENCY)
                    }
                    "UNRECOGNIZED" -> {
                        settingsBuilder
                            .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_LOW_LATENCY)
                    }
                }
                settingsBuilder
                    .setTxPowerLevel(AdvertiseSettings.ADVERTISE_TX_POWER_HIGH)
                settingsBuilder.setConnectable(true)

                bluetoothLeAdvertiser!!.startAdvertising(
                    settingsBuilder.build(),
                    dataBuilder.build(), advertiseCallback
                )
            } else {
                AppLog.e(
                    TAG,
                    "Peripheral not supported"
                )
            }
        }
        t.start()

    }

    /**
     * This Method Will Start the Service
     */
    fun start(
        context: Context
    ) {
        if (bleService == null) {
            val intent = Intent(context, BleService::class.java)
            context.startService(intent)
        } else {
            AppLog.e(TAG, "$TAG already started")
        }
    }

    /**
     * This Method Will Return True if Service is Running
     */
    fun isRunning(): Boolean {
        return bleService != null
    }

    /**
     * This Method Will Return True if Advertisement is ON
     */
    fun isAdvertiserRunning(): Boolean {
        return isAdvertisementRunning
    }

    /**
     * This Method Will Stop the Service if It Is Running
     */
    fun stop() {
        if (bleService != null) {
            var str = "$TAG stopped"
            bleService?.outOfRangeChecker?.removeCallbacks(outRangeRunnable)
            if (bleService!!.isAdvertiserRunning()) {
                bluetoothLeAdvertiser?.stopAdvertising(advertiseCallback)
                gattServer?.clearServices()
                gattServer?.close()
                str = str.plus(" Advertisement Stopped")
            }
            bleAdvertiseCallback?.stopAdvertiseRes(
                status = true,
                errorText = str
            )

            if (bleService!!.isScanRunning()) {
                stopScan()
            }
            bleCallback?.stopScanRes(status = true, errorText = "Scanning Stopped")
            bleService?.stopSelf()
        } else {
            bleAdvertiseCallback?.stopAdvertiseRes(
                status = false,
                errorText = "$TAG not started"
            )
            bleCallback?.stopScanRes(status = false, errorText = "")
            AppLog.e(TAG, "$TAG not started")
        }
    }

    /**
     * This Method Will Be Called When Scanning Is Failed
     */
    private fun onScanfailed(errorCode: Int) {
        var unknownError = false
        isScanningRunning = false
        var errorText = ""
        if (errorCode < 1 || errorCode > 4) {
            unknownError = true
        }
        when (errorCode) {
            1 -> {
                errorText = "SCAN_FAILED_ALREADY_STARTED"
            }
            2 -> {
                errorText = "SCAN_FAILED_APPLICATION_REGISTRATION_FAILED"
            }
            3 -> {
                errorText = "SCAN_FAILED_INTERNAL_ERROR"
            }
            4 -> {
                errorText = "SCAN_FAILED_FEATURE_UNSUPPORTED"
            }
        }
        val failMsg = "Scanning failed: $errorText"
        AppLog.e(TAG, failMsg)
        bleService?.bleCallback?.startScanRes(
            status = false,
            errorText = failMsg,
            unknownError = unknownError
        )
    }

    private fun setFilter(uuidList: ArrayList<ParcelUuid>) {
        for (uuid in uuidList) {
            filters.add(
                ScanFilter.Builder().setServiceUuid(uuid).build()
            )
        }
    }

    private fun parseBLEFrame(device: BluetoothDevice, rssi: Int, result: ScanResult) {
        AppLog.d(TAG, "device : " + device.address)

        if (result.scanRecord?.serviceUuids != null && result.scanRecord?.serviceUuids!!.contains(
                ParcelUuid.fromString(SERVICE_UUID)
            )
        ) {
            val selectItem = devicesList.find { it.macAddress == device.address }
            if (selectItem == null) {
                AppLog.d(TAG, "device : " + device.address)
                AppLog.d(TAG, "UUID : " + result.scanRecord!!.serviceUuids)
                RemoteLog[this]!!.addDebugLog("$TAG:device : " + device.address + " " + result.scanRecord!!.serviceUuids)
                val bleDevice: BLEScanDevice = BLEScanDevice.getDevice()
                bleDevice.bluetoothDevice = device
                bleDevice.scanResult = result
                bleDevice.name = device.name
                bleDevice.deviceRSSI = rssi
                bleDevice.macAddress = device.address
                bleDevice.isConnectable = result.isConnectable
                bleDevice.lastFoundTime = System.currentTimeMillis()
                devicesList.add(bleDevice)
                connectDevice(bleDevice)
            } else {
                selectItem.deviceRSSI = rssi
                selectItem.scanResult = result
                selectItem.name = device.name
                selectItem.isConnectable = result.isConnectable
                selectItem.lastFoundTime = System.currentTimeMillis()
            }
        }
    }

    private fun deviceFound(bleDevice: BLEScanDevice) {
        bleCallback?.deviceFound(bleDevice = bleDevice)
    }

    /**
     * This method will stop the scanning
     */
    fun stopScan() {
        AppLog.e(TAG, "stopScan()")
        isScanningRunning = false
        bleScanner.stopScan(scanCallback)
        RemoteLog[this]!!.addDebugLog("$TAG:Scanning Stopped")
    }


    private fun refreshDeviceCache(gatt: BluetoothGatt?) {
        try {
            val localMethod = gatt?.javaClass?.getMethod("refresh", *arrayOfNulls(0))
            localMethod?.invoke(gatt, *arrayOfNulls(0))
        } catch (localException: Exception) {
        }
    }

    /**
     * This Method Will Start the GattServer.
     */
    private fun startGattServer(service: BluetoothGattService) {
        gattServer = bluetoothManager!!.openGattServer(
            this,
            gattServerCallback
        )
        gattServer?.addService(service)
    }

    /**
     * This is a Object of a BluetoothGattServer with it's Callback.
     */
    private var gattServerCallback: BluetoothGattServerCallback =
        object : BluetoothGattServerCallback() {
            override fun onConnectionStateChange(
                device: BluetoothDevice, status: Int,
                newState: Int
            ) {
                super.onConnectionStateChange(device, status, newState)
            }

            override fun onServiceAdded(status: Int, service: BluetoothGattService) {
                super.onServiceAdded(status, service)
            }

            override fun onCharacteristicReadRequest(
                device: BluetoothDevice,
                requestId: Int, offset: Int,
                characteristic: BluetoothGattCharacteristic
            ) {
                super.onCharacteristicReadRequest(
                    device, requestId, offset,
                    characteristic
                )
                AppLog.e(TAG, "Request Received : " + qaulId?.size)
                gattServer?.sendResponse(
                    device,
                    requestId,
                    0,
                    0,
                    getStoredValue(characteristic)
                )
            }

            private fun getStoredValue(characteristic: BluetoothGattCharacteristic): ByteArray {
                val `val` = ByteArray(characteristic.value.size)
                System.arraycopy(
                    characteristic.value, 0, `val`, 0,
                    characteristic.value.size
                )

                return `val`
            }

            override fun onCharacteristicWriteRequest(
                device: BluetoothDevice,
                requestId: Int, characteristic: BluetoothGattCharacteristic,
                preparedWrite: Boolean, responseNeeded: Boolean, offset: Int,
                value: ByteArray
            ) {
                super.onCharacteristicWriteRequest(
                    device, requestId,
                    characteristic, preparedWrite, responseNeeded, offset,
                    value
                )
            }

            override fun onDescriptorReadRequest(
                device: BluetoothDevice,
                requestId: Int,
                offset: Int,
                descriptor: BluetoothGattDescriptor
            ) {
                super.onDescriptorReadRequest(device, requestId, offset, descriptor)
            }

            override fun onDescriptorWriteRequest(
                device: BluetoothDevice,
                requestId: Int,
                descriptor: BluetoothGattDescriptor,
                preparedWrite: Boolean,
                responseNeeded: Boolean,
                offset: Int,
                value: ByteArray
            ) {
                super.onDescriptorWriteRequest(
                    device,
                    requestId,
                    descriptor,
                    preparedWrite,
                    responseNeeded,
                    offset,
                    value
                )
            }

            override fun onExecuteWrite(device: BluetoothDevice, requestId: Int, execute: Boolean) {
                super.onExecuteWrite(device, requestId, execute)
            }

            override fun onNotificationSent(device: BluetoothDevice, status: Int) {
                super.onNotificationSent(device, status)
            }
        }

    /**
     * This is a Object of a AdvertiseCallback.
     */
    private val advertiseCallback: AdvertiseCallback = object : AdvertiseCallback() {
        override fun onStartSuccess(advertiseSettings: AdvertiseSettings) {
            val successMsg = "Advertisement successful"
            isAdvertisementRunning = true
            AppLog.e(TAG, successMsg)
            bleService?.bleAdvertiseCallback?.startAdvertiseRes(
                status = true,
                errorText = successMsg,
                unknownError = false
            )
        }

        override fun onStartFailure(i: Int) {
            var unknownError = false
            isAdvertisementRunning = false
            var errorText = ""
            if (i < 1 || i > 5) {
                unknownError = true
            }
            when (i) {
                1 -> {
                    errorText = "ADVERTISE_FAILED_DATA_TOO_LARGE"
                }
                2 -> {
                    errorText = "ADVERTISE_FAILED_TOO_MANY_ADVERTISERS"
                }
                3 -> {
                    errorText = "ADVERTISE_FAILED_ALREADY_STARTED"
                }
                4 -> {
                    errorText = "ADVERTISE_FAILED_INTERNAL_ERROR"
                }
                5 -> {
                    errorText = "ADVERTISE_FAILED_FEATURE_UNSUPPORTED"
                }
            }

            val failMsg = "Advertisement failed: $errorText"
            AppLog.e(TAG, failMsg)
            bleService?.bleAdvertiseCallback?.startAdvertiseRes(
                status = false,
                errorText = failMsg,
                unknownError = unknownError
            )
        }
    }

    /**
     * This method Will be Called When Service Will Stopped/Destroyed
     */
    override fun onDestroy() {
        if (bleService != null) {
            if (bleService!!.isAdvertiserRunning()) {
                bluetoothLeAdvertiser?.stopAdvertising(advertiseCallback)
                gattServer?.clearServices()
                gattServer?.close()
            }
            bleService?.outOfRangeChecker?.removeCallbacks(outRangeRunnable)
            stopScan()
            bleService?.stopSelf()
        }
        bleService = null
        super.onDestroy()
    }

    fun isScanRunning(): Boolean {
        return isScanningRunning
    }

    /**
     * This Method Will Set Filter, ScanMode, and Start Scanning
     */
    fun startScan(bleCallback: BleScanCallBack) {
        this.bleCallback = bleCallback
        if (bluetoothManager != null) {
            if (bluetoothAdapter != null) {
                bleScanner = bluetoothManager!!.adapter!!.bluetoothLeScanner
            } else {
                bluetoothAdapter = bluetoothManager!!.adapter
                bluetoothAdapter!!.name = "qaul"
                bleScanner = bluetoothAdapter!!.bluetoothLeScanner
            }
        } else {
            bluetoothManager = bleService!!.getSystemService(BLUETOOTH_SERVICE) as BluetoothManager
            bluetoothAdapter = bluetoothManager!!.adapter
            bluetoothAdapter!!.name = "qaul"
            bleScanner = bluetoothAdapter!!.bluetoothLeScanner
        }
        uuidList.clear()
        uuidList.add(ParcelUuid.fromString(SERVICE_UUID))
//        setFilter(uuidList)
        scanCallback = object : ScanCallback() {
            override fun onScanResult(callbackType: Int, result: ScanResult?) {
                super.onScanResult(callbackType, result)
                AppLog.d(TAG, "device : " + result!!.device.address)
                RemoteLog[this@BleService]!!.addDebugLog("$TAG:device : " + result.device.address)
                parseBLEFrame(result.device, result.rssi, result)
            }

            override fun onScanFailed(errorCode: Int) {
                super.onScanFailed(errorCode)
                onScanfailed(errorCode)
                stopScan()
            }
        }

        scanSettings =
            ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build()

        bleScanner.startScan(filters, scanSettings, scanCallback)
        if (!isScanRunning()) {
            bleService?.bleCallback?.startScanRes(
                status = true,
                errorText = "Scanning Started",
                unknownError = false
            )
            isScanningRunning = true
            startOutRangeChecker()
        }
    }

    private fun startOutRangeChecker() {
        outOfRangeChecker.postDelayed(outRangeRunnable, 2000)
    }


    private var outRangeRunnable: Runnable = Runnable {
        if (ignoreList.size > 0) {
            for (bLEDevice in ignoreList) {
                if (bLEDevice.lastFoundTime!! < System.currentTimeMillis() - 5000) {
                    bleCallback?.deviceOutOfRange(bleDevice = bLEDevice)
                    devicesList.remove(bLEDevice)
                    ignoreList.remove(bLEDevice)
                } else {
                    AppLog.e(TAG, "Still in range")
                }
            }
        }
        startOutRangeChecker()
    }

    private fun connectDevice(device: BLEScanDevice) {
        val baseBleActor = BaseBleActor(this, object : BaseBleActor.BleConnectionListener {
            override fun onConnected(macAddress: String?) {
                AppLog.e(TAG, " onConnected : $macAddress")
            }

            override fun onDisconnected(macAddress: String?) {
                AppLog.e(TAG, " onDisconnected : $macAddress")
            }

            override fun onServiceDiscovered(macAddress: String?) {
                AppLog.e(TAG, " onServiceDiscovered : $macAddress")
            }

            override fun onDescriptorWrite(bleScanDevice: BLEScanDevice) {
                AppLog.e(TAG, " onDescriptorWrite : ${bleScanDevice.macAddress}")
                deviceFound(bleScanDevice)
            }

            override fun onConnectionFailed(macAddress: String?) {
                AppLog.e(TAG, " onConnectionFailed : $macAddress")
            }

            override fun onCharacteristicRead(
                macAddress: String?,
                gatt: BluetoothGatt?,
                characteristic: BluetoothGattCharacteristic?
            ) {

            }

            override fun onCharacteristicWrite(
                gatt: BluetoothGatt?,
                characteristic: BluetoothGattCharacteristic?
            ) {

            }

            override fun onCharacteristicChanged(
                macAddress: String?,
                gatt: BluetoothGatt?,
                characteristic: BluetoothGattCharacteristic?
            ) {

            }

            override fun addToBlackList(bleScanDevice: BLEScanDevice) {
                blackList.add(bleScanDevice)
                AppLog.e(TAG, " addToBlackList : ${blackList.toString()}")
            }

            override fun addToIgnoreList(bleScanDevice: BLEScanDevice) {
                ignoreList.add(bleScanDevice)
                AppLog.e(TAG, " addToIgnoreList : ${ignoreList.toString()}")
            }

        })
        baseBleActor.setDevice(device)
    }

    /**
     * This is a Interface for Sending Advertisement Start & Stop Response to BLEWrapperClass.
     */
    interface BleAdvertiseCallback {
        fun startAdvertiseRes(status: Boolean, errorText: String, unknownError: Boolean)
        fun stopAdvertiseRes(status: Boolean, errorText: String)
    }

    /**
     * This is a Interface for Sending Scan Start & Stop Response to BLEWrapperClass.
     */
    interface BleScanCallBack {
        fun startScanRes(status: Boolean, errorText: String, unknownError: Boolean)
        fun stopScanRes(status: Boolean, errorText: String)
        fun deviceFound(bleDevice: BLEScanDevice)
        fun deviceOutOfRange(bleDevice: BLEScanDevice)
    }

}