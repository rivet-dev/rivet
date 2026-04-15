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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTransporter6.js
var require_faTransporter6 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTransporter6.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "transporter-6";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e2a7";
    var svgPathData = "M56.7 6.6L64 32l25.4 7.3C93.3 40.4 96 43.9 96 48s-2.7 7.6-6.6 8.7L64 64 56.7 89.4C55.6 93.3 52.1 96 48 96s-7.6-2.7-8.7-6.6L32 64 6.6 56.7C2.7 55.6 0 52.1 0 48s2.7-7.6 6.6-8.7L32 32 39.3 6.6C40.4 2.7 43.9 0 48 0s7.6 2.7 8.7 6.6zM384 512l-88 0-80 0-88 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l256 0c17.7 0 32 14.3 32 32s-14.3 32-32 32zm96-128l25.4 7.3c3.9 1.1 6.6 4.7 6.6 8.7s-2.7 7.6-6.6 8.7L480 416l-7.3 25.4c-1.1 3.9-4.7 6.6-8.7 6.6s-7.6-2.7-8.7-6.6L448 416l-25.4-7.3c-3.9-1.1-6.6-4.7-6.6-8.7s2.7-7.6 6.6-8.7L448 384l7.3-25.4c1.1-3.9 4.7-6.6 8.7-6.6s7.6 2.7 8.7 6.6L480 384zM224 0l64 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-64 0c-8.8 0-16-7.2-16-16s7.2-16 16-16zM112 256l288 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-288 0c-8.8 0-16-7.2-16-16s7.2-16 16-16z";
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
    exports.faTransporter6 = exports.definition;
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

// src/_entries/faTransporter6.js
var import_faTransporter6 = __toESM(require_faTransporter6());
var export_faTransporter6 = import_faTransporter6.definition;
export {
  export_faTransporter6 as faTransporter6
};
