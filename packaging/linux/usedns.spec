Name:           usedns
Version:        %{version}
Release:        %{release}%{?dist}
Summary:        Desktop utility for choosing DNS resolvers
License:        MIT
URL:            https://github.com/x-labs-myid/UseDNS
BuildArch:      x86_64

Requires:       fontconfig
Requires:       freetype
Requires:       libxkbcommon
Requires:       libxcb

%description
UseDNS is a lightweight desktop application for comparing public DNS
providers. Changing system DNS is currently supported on Windows only;
the Linux package provides the application UI.

%install
rm -rf %{buildroot}
install -Dpm0755 %{_sourcedir}/usedns %{buildroot}%{_bindir}/usedns
install -Dpm0644 %{_sourcedir}/usedns.desktop %{buildroot}%{_datadir}/applications/usedns.desktop
install -Dpm0644 %{_sourcedir}/usedns.png %{buildroot}%{_datadir}/pixmaps/usedns.png
install -Dpm0644 %{_sourcedir}/LICENSE %{buildroot}%{_licensedir}/usedns/LICENSE
install -Dpm0644 %{_sourcedir}/README.md %{buildroot}%{_docdir}/usedns/README.md

%files
%{_bindir}/usedns
%{_datadir}/applications/usedns.desktop
%{_datadir}/pixmaps/usedns.png
%license %{_licensedir}/usedns/LICENSE
%doc %{_docdir}/usedns/README.md

%changelog
* Thu Jan 01 2026 UseDNS <usedns@x-labs.my.id> - %{version}-%{release}
- Automated GitHub release build
