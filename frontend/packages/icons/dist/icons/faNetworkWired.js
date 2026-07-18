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

// src/node_modules/@fortawesome/free-solid-svg-icons/faNetworkWired.js
var require_faNetworkWired = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faNetworkWired.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "network-wired";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f6ff";
    var svgPathData = "M248 88l80 0 0 48-80 0 0-48zm-8-56c-26.5 0-48 21.5-48 48l0 64c0 26.5 21.5 48 48 48l16 0 0 32-224 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l96 0 0 32-16 0c-26.5 0-48 21.5-48 48l0 64c0 26.5 21.5 48 48 48l96 0c26.5 0 48-21.5 48-48l0-64c0-26.5-21.5-48-48-48l-16 0 0-32 192 0 0 32-16 0c-26.5 0-48 21.5-48 48l0 64c0 26.5 21.5 48 48 48l96 0c26.5 0 48-21.5 48-48l0-64c0-26.5-21.5-48-48-48l-16 0 0-32 96 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-224 0 0-32 16 0c26.5 0 48-21.5 48-48l0-64c0-26.5-21.5-48-48-48l-96 0zM448 376l8 0 0 48-80 0 0-48 72 0zm-256 0l8 0 0 48-80 0 0-48 72 0z";
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
    exports.faNetworkWired = exports.definition;
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

// src/_entries/faNetworkWired.js
var import_faNetworkWired = __toESM(require_faNetworkWired());
var export_faNetworkWired = import_faNetworkWired.definition;
export {
  export_faNetworkWired as faNetworkWired
};
