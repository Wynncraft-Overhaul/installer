export DESTINATION = macos_bundle
mkdir $DESTINATION/
mkdir $DESTINATION/Installer.app/
mkdir $DESTINATION/Installer.app/Contents/
mkdir $DESTINATION/Installer.app/Contents/Resources
mkdir $DESTINATION/Installer.app/Contents/MacOS/
cp icon_256.icns $DESTINATION/Installer.app/Contents/Resources/icon_256.icns
cp target/debug/installer $DESTINATION/Installer.app/Contents/MacOS/installer
cat > $DESTINATION/Installer.app/Contents/Info.plist << EOF
{
   CFBundleName = installer;
   CFBundleDisplayName = Majestic Overhaul Installer;
   CFBundleIdentifier = "io.github.wynncraft-overhaul";
   CFBundleVersion = "1.0.0";
   CFBundleShortVersionString = "1.0.0";
   CFBundleInfoDictionaryVersion = "6.0";
   CFBundlePackageType = APPL;
   CFBundleSignature = inst;
   CFBundleExecutable = installer;
   CFBundleIconFile = "icon_256.icns";
}
EOF
