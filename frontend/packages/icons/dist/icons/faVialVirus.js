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

// src/node_modules/@fortawesome/free-solid-svg-icons/faVialVirus.js
var require_faVialVirus = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faVialVirus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "vial-virus";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e597";
    var svgPathData = "M64 32C64 14.3 78.3 0 96 0L320 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l0 151.7c-18.5 0-37 7-51.1 21.1-21.6 21.6-26.6 53.6-15 79.9-26.9 10.4-45.9 36.6-45.9 67.1s19 56.7 45.9 67.1c-7.3 16.5-8 35.2-2.3 52.2-13.4 5.7-28.2 8.8-43.6 8.8-61.9 0-112-50.1-112-112L96 64C78.3 64 64 49.7 64 32zm96 32l0 128 96 0 0-128-96 0zM280 408c-13.3 0-24-10.7-24-24s10.7-24 24-24c28.8 0 43.2-34.8 22.9-55.2-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0c20.4 20.4 55.2 5.9 55.2-22.9 0-13.3 10.7-24 24-24s24 10.7 24 24c0 28.8 34.8 43.2 55.2 22.9 9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9c-20.4 20.4-5.9 55.2 22.9 55.2 13.3 0 24 10.7 24 24s-10.7 24-24 24c-28.8 0-43.2 34.8-22.9 55.2 9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0c-20.4-20.4-55.2-5.9-55.2 22.9 0 13.3-10.7 24-24 24s-24-10.7-24-24c0-28.8-34.8-43.2-55.2-22.9-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9c20.4-20.4 5.9-55.2-22.9-55.2zm104-32a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm88 40a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faVialVirus = exports.definition;
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

// src/_entries/faVialVirus.js
var import_faVialVirus = __toESM(require_faVialVirus());
var export_faVialVirus = import_faVialVirus.definition;
export {
  export_faVialVirus as faVialVirus
};
