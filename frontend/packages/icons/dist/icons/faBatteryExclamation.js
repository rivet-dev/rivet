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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBatteryExclamation.js
var require_faBatteryExclamation = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBatteryExclamation.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "battery-exclamation";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e0b0";
    var svgPathData = "M208 96l0 64L80 160c-8.8 0-16 7.2-16 16l0 160c0 8.8 7.2 16 16 16l118.7 0c-4.3 9.8-6.7 20.6-6.7 32s2.4 22.2 6.7 32L80 416c-44.2 0-80-35.8-80-80L0 176c0-44.2 35.8-80 80-80l128 0zM464 416l-118.7 0c4.3-9.8 6.7-20.6 6.7-32s-2.4-22.2-6.7-32L464 352c8.8 0 16-7.2 16-16l0-160c0-8.8-7.2-16-16-16l-128 0 0-64 128 0c44.2 0 80 35.8 80 80l0 16c17.7 0 32 14.3 32 32l0 64c0 17.7-14.3 32-32 32l0 16c0 44.2-35.8 80-80 80zM272 96c13.3 0 24 10.7 24 24l0 176c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-176c0-13.3 10.7-24 24-24zM240 384a32 32 0 1 1 64 0 32 32 0 1 1 -64 0z";
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
    exports.faBatteryExclamation = exports.definition;
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

// src/_entries/faBatteryExclamation.js
var import_faBatteryExclamation = __toESM(require_faBatteryExclamation());
var export_faBatteryExclamation = import_faBatteryExclamation.definition;
export {
  export_faBatteryExclamation as faBatteryExclamation
};
