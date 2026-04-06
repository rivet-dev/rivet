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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCoffin.js
var require_faCoffin = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCoffin.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "coffin";
    var width = 384;
    var height = 512;
    var aliases = [9904];
    var unicode = "f6c6";
    var svgPathData = "M11.5 114.9l91-104C108.5 4 117.3 0 126.5 0h131c9.2 0 18 4 24.1 10.9l91 104c7.4 8.5 11.5 19.3 11.5 30.6c0 4.4-.6 8.7-1.8 12.9L286.7 488.9c-4 13.7-16.5 23.1-30.7 23.1H128.1c-14.3 0-26.8-9.4-30.7-23.1L1.8 158.3C.6 154.1 0 149.8 0 145.4c0-11.2 4.1-22.1 11.5-30.6z";
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
    exports.faCoffin = exports.definition;
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

// src/_entries/faCoffin.js
var import_faCoffin = __toESM(require_faCoffin());
var export_faCoffin = import_faCoffin.definition;
export {
  export_faCoffin as faCoffin
};
