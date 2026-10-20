Pod::Spec.new do |s|
  s.name = 'Widgets'
  s.version = '1.0.0'
  s.summary = 'TypeAside widget snapshot bridge'
  s.author = 'TypeAside'
  s.homepage = 'https://typeaside.app'
  s.platforms = { :ios => '17.0' }
  s.swift_version = '5.9'
  s.source = { git: '' }
  s.static_framework = true
  s.dependency 'ExpoModulesCore'
  s.frameworks = 'WidgetKit', 'SwiftUI'
  s.source_files = '**/*.swift'
end
