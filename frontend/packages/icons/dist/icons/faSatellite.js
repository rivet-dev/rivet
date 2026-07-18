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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSatellite.js
var require_faSatellite = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSatellite.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "satellite";
    var width = 512;
    var height = 512;
    var aliases = [128752];
    var unicode = "f7bf";
    var svgPathData = "M199 7c9.4-9.4 24.6-9.4 33.9 0l89.4 89.4 55-55c12.5-12.5 32.8-12.5 45.3 0l48 48c12.5 12.5 12.5 32.8 0 45.3l-55 55 89.4 89.4c9.4 9.4 9.4 24.6 0 33.9l-96 96c-9.4 9.4-24.6 9.4-33.9 0l-89.4-89.4-15.5 15.5c11.4 24.6 17.8 52 17.8 80.9 0 31.7-7.7 61.5-21.2 87.8-4.7 9-16.7 10.3-23.8 3.1l-96.3-96.3-60 60c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3l60-60-96.3-96.3c-7.2-7.2-5.9-19.2 3.1-23.8 26.3-13.6 56.2-21.2 87.8-21.2 28.9 0 56.3 6.4 80.9 17.8L192.4 226.3 103 137c-9.4-9.4-9.4-24.6 0-33.9L199 7zm17 50.9l-62.1 62.1 72.4 72.4 62.1-62.1-72.4-72.4zM392 358.1l62.1-62.1-72.4-72.4-62.1 62.1 72.4 72.4z";
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
    exports.faSatellite = exports.definition;
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

// src/_entries/faSatellite.js
var import_faSatellite = __toESM(require_faSatellite());
var export_faSatellite = import_faSatellite.definition;
export {
  export_faSatellite as faSatellite
};
