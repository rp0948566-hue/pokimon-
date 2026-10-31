"""Candidate architectures. These are NEW transfer-learning candidates, not the deployed models."""
from __future__ import annotations

import os
from pathlib import Path

# Licensed pretrained weights live inside the lab, not in ~/.cache or a production repo.
os.environ.setdefault("HF_HOME", str(Path(__file__).resolve().parents[2] / "models/pretrained/hf"))

import torch  # noqa: E402
import torch.nn as nn  # noqa: E402


def build_grading_model(name: str = "convnextv2_nano.fcmae_ft_in22k_in1k", num_classes: int = 5,
                        pretrained: bool = True, drop_rate: float = 0.2) -> nn.Module:
    import timm

    return timm.create_model(name, pretrained=pretrained, num_classes=num_classes, drop_rate=drop_rate)


def set_backbone_trainable(model: nn.Module, trainable: bool) -> int:
    """Freeze/unfreeze everything except the classifier head. Returns trainable param count."""
    head = {id(p) for p in model.get_classifier().parameters()}
    for p in model.parameters():
        p.requires_grad = trainable or id(p) in head
    return sum(p.numel() for p in model.parameters() if p.requires_grad)


class UNet(nn.Module):
    """U-Net decoder on a timm features_only encoder. Output: 1-channel logits at input resolution."""

    def __init__(self, encoder: str = "resnet34", pretrained: bool = True, out_ch: int = 1):
        super().__init__()
        import timm

        self.encoder = timm.create_model(encoder, pretrained=pretrained, features_only=True)
        chs = self.encoder.feature_info.channels()
        self.up = nn.ModuleList()
        prev = chs[-1]
        for skip in reversed(chs[:-1]):
            self.up.append(nn.Sequential(nn.Conv2d(prev + skip, skip, 3, padding=1, bias=False),
                                         nn.BatchNorm2d(skip), nn.ReLU(inplace=True),
                                         nn.Conv2d(skip, skip, 3, padding=1, bias=False),
                                         nn.BatchNorm2d(skip), nn.ReLU(inplace=True)))
            prev = skip
        self.head = nn.Conv2d(prev, out_ch, 1)

    def get_classifier(self) -> nn.Module:  # lets set_backbone_trainable treat the 1x1 conv as the "head"
        return self.head

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        size = x.shape[-2:]
        feats = self.encoder(x)
        y = feats[-1]
        for blk, skip in zip(self.up, reversed(feats[:-1])):
            y = nn.functional.interpolate(y, size=skip.shape[-2:], mode="bilinear", align_corners=False)
            y = blk(torch.cat([y, skip], 1))
        return nn.functional.interpolate(self.head(y), size=size, mode="bilinear", align_corners=False)


def build_seg_model(encoder: str = "resnet34", pretrained: bool = True) -> nn.Module:
    return UNet(encoder, pretrained)
