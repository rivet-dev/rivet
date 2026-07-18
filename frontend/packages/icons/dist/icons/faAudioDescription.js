var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/free-solid-svg-icons/faAudioDescription.js
var require_faAudioDescription = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faAudioDescription.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "audio-description";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f29e";
    var svgPathData = "M0 128C0 92.7 28.7 64 64 64l384 0c35.3 0 64 28.7 64 64l0 256c0 35.3-28.7 64-64 64L64 448c-35.3 0-64-28.7-64-64L0 128zm96 96l0 104c0 13.3 10.7 24 24 24s24-10.7 24-24l0-24 48 0 0 24c0 13.3 10.7 24 24 24s24-10.7 24-24l0-104c0-35.3-28.7-64-64-64l-16 0c-35.3 0-64 28.7-64 64zm48 32l0-32c0-8.8 7.2-16 16-16l16 0c8.8 0 16 7.2 16 16l0 32-48 0zm152-96c-13.3 0-24 10.7-24 24l0 144c0 13.3 10.7 24 24 24l48 0c39.8 0 72-32.2 72-72l0-48c0-39.8-32.2-72-72-72l-48 0zm48 144l-24 0 0-96 24 0c13.3 0 24 10.7 24 24l0 48c0 13.3-10.7 24-24 24z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faAudioDescription = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faAudioDescription.js
var import_faAudioDescription = __toESM(require_faAudioDescription());
var export_faAudioDescription = import_faAudioDescription.definition;
export {
  export_faAudioDescription as faAudioDescription
};
