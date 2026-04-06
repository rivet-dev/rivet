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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHouseChimneyWindow.js
var require_faHouseChimneyWindow = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHouseChimneyWindow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "house-chimney-window";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e00d";
    var svgPathData = "M234.2 8.6c12.3-11.4 31.3-11.4 43.5 0L368 92.3 368 80c0-17.7 14.3-32 32-32l32 0c17.7 0 32 14.3 32 32l0 101.5 37.8 35.1c9.6 9 12.8 22.9 8 35.1S493.2 272 480 272l-16 0 0 176c0 35.3-28.7 64-64 64l-288 0c-35.3 0-64-28.7-64-64l0-176-16 0c-13.2 0-25-8.1-29.8-20.3s-1.6-26.2 8-35.1l224-208zM216 224c-13.3 0-24 10.7-24 24l0 80c0 13.3 10.7 24 24 24l80 0c13.3 0 24-10.7 24-24l0-80c0-13.3-10.7-24-24-24l-80 0z";
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
    exports.faHouseChimneyWindow = exports.definition;
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

// src/_entries/faHouseChimneyWindow.js
var import_faHouseChimneyWindow = __toESM(require_faHouseChimneyWindow());
var export_faHouseChimneyWindow = import_faHouseChimneyWindow.definition;
export {
  export_faHouseChimneyWindow as faHouseChimneyWindow
};
