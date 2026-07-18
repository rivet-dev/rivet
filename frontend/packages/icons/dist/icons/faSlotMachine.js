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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSlotMachine.js
var require_faSlotMachine = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSlotMachine.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "slot-machine";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e3ce";
    var svgPathData = "M128 0C92.7 0 64 28.7 64 64l384 0c0-35.3-28.7-64-64-64L128 0zM64 96C28.7 96 0 124.7 0 160L0 352c0 35.3 28.7 64 64 64l0 32c0 35.3 28.7 64 64 64l288 0 32 0 80 0c53 0 96-43 96-96l0-204.2c9.8-8.8 16-21.6 16-35.8c0-26.5-21.5-48-48-48s-48 21.5-48 48c0 14.2 6.2 27 16 35.8L560 416c0 17.7-14.3 32-32 32l-80 0 0-32c35.3 0 64-28.7 64-64l0-192c0-35.3-28.7-64-64-64L64 96zM56 200c0-13.3 10.7-24 24-24l59.9 0c23 0 38.5 23.5 29.4 44.6L126.1 321.5c-5.2 12.2-19.3 17.8-31.5 12.6s-17.8-19.3-12.6-31.5L115.6 224 80 224c-13.3 0-24-10.7-24-24zm168-24l59.9 0c23 0 38.5 23.5 29.4 44.6L270.1 321.5c-5.2 12.2-19.3 17.8-31.5 12.6s-17.8-19.3-12.6-31.5L259.6 224 224 224c-13.3 0-24-10.7-24-24s10.7-24 24-24zm120 24c0-13.3 10.7-24 24-24l59.9 0c23 0 38.5 23.5 29.4 44.6L414.1 321.5c-5.2 12.2-19.3 17.8-31.5 12.6s-17.8-19.3-12.6-31.5L403.6 224 368 224c-13.3 0-24-10.7-24-24z";
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
    exports.faSlotMachine = exports.definition;
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

// src/_entries/faSlotMachine.js
var import_faSlotMachine = __toESM(require_faSlotMachine());
var export_faSlotMachine = import_faSlotMachine.definition;
export {
  export_faSlotMachine as faSlotMachine
};
