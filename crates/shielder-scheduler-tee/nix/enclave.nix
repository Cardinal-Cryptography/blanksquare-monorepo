{ buildEnv
, busybox
, nitro # when you call this function pass `nitro-util.lib.${system}` here
, stdenv
, crossArch
, dockerImageFs
}:

let
  # Extract the Docker image to get the rootfs
  dockerRootfs = stdenv.mkDerivation {
    name = "docker-rootfs";
    src = dockerImageFs;
    
    buildPhase = ''
      # Extract the Docker image layers
      mkdir -p rootfs
      tar -xf ${dockerImageFs} -C .
      
      # Docker images have layers, we need to merge them
      if [ -d "." ]; then
        # Copy all extracted content to rootfs
        find . -mindepth 1 -maxdepth 1 -not -name rootfs -exec cp -r {} rootfs/ \;
      fi
    '';
    
    installPhase = ''
      cp -r rootfs $out
    '';
  };
in

nitro.buildEif {
  arch = crossArch;
  kernel = nitro.blobs.${crossArch}.kernel;
  kernelConfig = nitro.blobs.${crossArch}.kernelConfig;

  name = "shielder-scheduler-tee-eif-${crossArch}";

  nsmKo = nitro.blobs.${crossArch}.nsmKo;

  # Use the Docker image rootfs instead of building our own
  copyToRoot = dockerRootfs;

  # The entrypoint should be whatever is defined in the Docker image
  # or the binary path within the image
  entrypoint = ''
    /usr/local/bin/shielder-scheduler-tee
  '';

  env = "";
}
