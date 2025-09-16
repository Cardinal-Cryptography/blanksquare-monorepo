{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    nitro-util.url = "github:monzo/aws-nitro-util";
    nitro-util.inputs.nixpkgs.follows = "nixpkgs";

    flake-utils.url = "github:numtide/flake-utils";
  };
  
  outputs = { nitro-util, nixpkgs, flake-utils, ... }: (flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs { inherit system; };
      nitro = nitro-util.lib.${system};
    in
    rec {
      defaultPackage = packages.shielderSchedulerTEE;

      packages = {
        shielderSchedulerTEE =
          let
            crossArch = "x86_64";
            crossPkgs = import nixpkgs { inherit system; crossSystem = "${crossArch}-linux"; };
            
            # ECR Docker image configuration
            ecrImage = builtins.getEnv "ECR_IMAGE";
            defaultImage = "public.ecr.aws/p5g6f1p8/shielder-scheduler:latest"; 
            dockerImage = if ecrImage != "" then ecrImage else defaultImage;
            
            # Convert Docker image to a filesystem that nitro-util can use
            dockerImageFs = pkgs.dockerTools.pullImage {
              imageName = dockerImage;
              imageDigest = null; # Let it pull the latest
              sha256 = null;      # Will be determined at build time
              finalImageName = "shielder-scheduler-tee";
              finalImageTag = "latest";
            };
          in
          crossPkgs.callPackage ./enclave.nix {
            inherit crossArch nitro;
            # Pass the Docker image filesystem directly
            dockerImageFs = dockerImageFs;
          };
      };
    }));
}
