from __future__ import annotations

from unittest.mock import patch

from app.cli import main
from app.config import ServerConfig, Settings


class TestCli:
    @patch("app.cli.uvicorn")
    @patch("app.cli.load_config", return_value=None)
    def test_defaults_when_no_config(self, mock_load_config, mock_uvicorn):
        with patch("app.cli._parse_args") as mock_args:
            mock_args.return_value.host = None
            mock_args.return_value.port = None
            mock_args.return_value.config = "config.yaml"

            main()

            mock_load_config.assert_called_once_with("config.yaml")
            mock_uvicorn.run.assert_called_once_with(
                "app.main:app", host="0.0.0.0", port=8000
            )

    @patch("app.cli.uvicorn")
    @patch("app.cli.load_config")
    def test_uses_config_values(self, mock_load_config, mock_uvicorn):
        mock_load_config.return_value = Settings(
            server=ServerConfig(host="127.0.0.1", port=9000)
        )
        with patch("app.cli._parse_args") as mock_args:
            mock_args.return_value.host = None
            mock_args.return_value.port = None
            mock_args.return_value.config = "custom.yaml"

            main()

            mock_load_config.assert_called_once_with("custom.yaml")
            mock_uvicorn.run.assert_called_once_with(
                "app.main:app", host="127.0.0.1", port=9000
            )

    @patch("app.cli.uvicorn")
    @patch("app.cli.load_config")
    def test_cli_args_override_config(self, mock_load_config, mock_uvicorn):
        mock_load_config.return_value = Settings(
            server=ServerConfig(host="127.0.0.1", port=9000)
        )
        with patch("app.cli._parse_args") as mock_args:
            mock_args.return_value.host = "10.0.0.1"
            mock_args.return_value.port = 3000
            mock_args.return_value.config = "config.yaml"

            main()

            mock_uvicorn.run.assert_called_once_with(
                "app.main:app", host="10.0.0.1", port=3000
            )
