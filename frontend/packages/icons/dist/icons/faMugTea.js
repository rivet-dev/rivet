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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMugTea.js
var require_faMugTea = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMugTea.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mug-tea";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f875";
    var svgPathData = "M144 64l0 50.7c0 8.5-3.4 16.6-9.4 22.6l-29.3 29.3c-6 6-9.4 14.1-9.4 22.6L96 256c0 17.7 14.3 32 32 32l64 0c17.7 0 32-14.3 32-32l0-66.7c0-8.5-3.4-16.6-9.4-22.6l-29.3-29.3c-6-6-9.4-14.1-9.4-22.6L176 64l208 0 64 0c70.7 0 128 57.3 128 128s-57.3 128-128 128l-32 0 0 32c0 53-43 96-96 96l-192 0c-53 0-96-43-96-96L32 96c0-17.7 14.3-32 32-32l80 0zM416 256l32 0c35.3 0 64-28.7 64-64s-28.7-64-64-64l-32 0 0 128z";
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
    exports.faMugTea = exports.definition;
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

// src/_entries/faMugTea.js
var import_faMugTea = __toESM(require_faMugTea());
var export_faMugTea = import_faMugTea.definition;
export {
  export_faMugTea as faMugTea
};
