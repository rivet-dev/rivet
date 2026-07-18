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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSeat.js
var require_faPersonSeat = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSeat.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-seat";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e21e";
    var svgPathData = "M144 128a48 48 0 1 0 0-96 48 48 0 1 0 0 96zm48 160l0-32 64 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-66.7 0c-6.6-18.6-24.4-32-45.3-32c-26.5 0-48 21.5-48 48l0 80c0 35.3 28.7 64 64 64l85.6 0c8.1 0 14.9 6 15.9 14l10.7 86c2 16 15.6 28 31.8 28l48 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-19.8 0L325 358.1c-5-40-39-70.1-79.4-70.1L192 288zM64 160c0-17.7-14.3-32-32-32s-32 14.3-32 32L0 320c0 70.7 57.3 128 128 128l64 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-64 0c-35.3 0-64-28.7-64-64l0-160z";
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
    exports.faPersonSeat = exports.definition;
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

// src/_entries/faPersonSeat.js
var import_faPersonSeat = __toESM(require_faPersonSeat());
var export_faPersonSeat = import_faPersonSeat.definition;
export {
  export_faPersonSeat as faPersonSeat
};
