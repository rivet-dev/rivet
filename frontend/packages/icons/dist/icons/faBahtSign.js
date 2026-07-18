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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBahtSign.js
var require_faBahtSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBahtSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "baht-sign";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "e0ac";
    var svgPathData = "M136 0c-13.3 0-24 10.7-24 24l0 40-74.4 0C16.8 64 0 80.8 0 101.6L0 406.3c0 23 18.7 41.7 41.7 41.7l70.3 0 0 40c0 13.3 10.7 24 24 24s24-10.7 24-24l0-40 48 0c61.9 0 112-50.1 112-112 0-40.1-21.1-75.3-52.7-95.1 13.1-18.3 20.7-40.7 20.7-64.9 0-61.9-50.1-112-112-112l-16 0 0-40c0-13.3-10.7-24-24-24zM112 128l0 96-48 0 0-96 48 0zm48 96l0-96 16 0c26.5 0 48 21.5 48 48s-21.5 48-48 48l-16 0zm-48 64l0 96-48 0 0-96 48 0zm48 96l0-96 48 0c26.5 0 48 21.5 48 48s-21.5 48-48 48l-48 0z";
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
    exports.faBahtSign = exports.definition;
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

// src/_entries/faBahtSign.js
var import_faBahtSign = __toESM(require_faBahtSign());
var export_faBahtSign = import_faBahtSign.definition;
export {
  export_faBahtSign as faBahtSign
};
