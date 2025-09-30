#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

const BUILD_GRADLE_PATH = path.join(__dirname, '../android/build.gradle');

function patchBuildGradle() {
  if (!fs.existsSync(BUILD_GRADLE_PATH)) {
    console.error(
      'Error: android/build.gradle not found. Make sure to run ubrn:android first.'
    );
    process.exit(1);
  }

  let content = fs.readFileSync(BUILD_GRADLE_PATH, 'utf8');

  // Check if already patched
  if (content.includes('-DANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON')) {
    console.log(
      '✅ build.gradle already patched with ANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON'
    );
    return;
  }

  // Find and replace the arguments line in externalNativeBuild
  const originalPattern = /(\s+arguments\s+)'-DANDROID_STL=c\+\+_shared'(\s+)/;
  const replacement =
    "$1'-DANDROID_STL=c++_shared',\n                  '-DANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON'$2";

  if (originalPattern.test(content)) {
    content = content.replace(originalPattern, replacement);
    fs.writeFileSync(BUILD_GRADLE_PATH, content);
    console.log(
      '✅ Successfully patched android/build.gradle with ANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON'
    );
  } else {
    console.error(
      '❌ Could not find expected pattern in build.gradle. The file structure may have changed.'
    );
    console.error('Please check the externalNativeBuild section manually.');
    process.exit(1);
  }
}

if (require.main === module) {
  patchBuildGradle();
}

module.exports = { patchBuildGradle };
