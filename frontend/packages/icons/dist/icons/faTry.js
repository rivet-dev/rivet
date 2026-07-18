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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTurkishLiraSign.js
var require_faTurkishLiraSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTurkishLiraSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "turkish-lira-sign";
    var width = 448;
    var height = 512;
    var aliases = ["try", "turkish-lira"];
    var unicode = "e2bb";
    var svgPathData = "M160 32c17.7 0 32 14.3 32 32l0 43.6 121.4-34.7c12.7-3.6 26 3.7 29.7 16.5s-3.7 26-16.5 29.7l-134.6 38.5 0 46.1 121.4-34.7c12.7-3.6 26 3.7 29.7 16.5s-3.7 26-16.5 29.7l-134.6 38.5 0 162.5 72 0c53 0 96-43 96-96 0-17.7 14.3-32 32-32s32 14.3 32 32c0 88.4-71.6 160-160 160l-104 0c-17.7 0-32-14.3-32-32l0-176.2-25.4 7.3c-12.7 3.6-26-3.7-29.7-16.5s3.7-26 16.5-29.7l38.6-11 0-46.1-25.4 7.3c-12.7 3.6-26-3.7-29.7-16.5s3.7-26 16.5-29.7l38.6-11 0-61.9c0-17.7 14.3-32 32-32z";
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
    exports.faTurkishLiraSign = exports.definition;
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

// src/_entries/faTry.js
var import_faTurkishLiraSign = __toESM(require_faTurkishLiraSign());
var export_faTry = import_faTurkishLiraSign.definition;
export {
  export_faTry as faTry
};
