rule Linux_Netcat_Reverse_Shell {
    meta:
        description = "Netcat execution with interactive command shell redirection"
        mitre_technique = "T1059.004"
        severity = "critical"
        threat_score = 95
        author = "Florian Roth / Neo23x0"
    strings:
        $nc1 = "nc -e /bin/sh" ascii
        $nc2 = "nc -e /bin/bash" ascii
        $nc3 = "ncat -e /bin/sh" ascii
        $nc4 = "ncat -e /bin/bash" ascii
        $nc5 = "nc.traditional -e" ascii
        $nc6 = "nc -c /bin/bash" ascii
    condition:
        any of them
}

rule Linux_Curl_Pipe_Shell_Downloader {
    meta:
        description = "Ingress tool download piped directly to shell interpreter"
        mitre_technique = "T1105"
        severity = "high"
        threat_score = 85
        author = "Aigis Security Operations"
    strings:
        $curl = "curl" ascii
        $wget = "wget" ascii
        $sh = "| sh" ascii
        $bash = "| bash" ascii
    condition:
        ($curl or $wget) and ($sh or $bash)
}

rule Linux_Base64_Encoded_Shell {
    meta:
        description = "Deobfuscation of base64 payload piped directly to shell execution"
        mitre_technique = "T1027"
        severity = "high"
        threat_score = 85
        author = "Florian Roth / Nextron Systems"
    strings:
        $b64_1 = "base64 -d | sh" ascii
        $b64_2 = "base64 -d | bash" ascii
        $b64_3 = "base64 --decode | sh" ascii
        $b64_4 = "base64 --decode | bash" ascii
    condition:
        any of them
}
