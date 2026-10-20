Pod::Spec.new do |s|
  s.name           = 'Keyboard'
  s.version        = '1.0.0'
  s.summary        = 'TypeAside keyboard bridge'
  s.author         = 'TypeAside'
  s.homepage       = 'https://typeaside.app'
  s.platforms      = { :ios => '16.4' }
  s.swift_version  = '5.9'
  s.source         = { git: '' }
  s.static_framework = true

  s.dependency 'ExpoModulesCore'

  s.source_files = '**/*.swift'
end
