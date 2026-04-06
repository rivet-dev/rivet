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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faConveyorBeltArm.js
var require_faConveyorBeltArm = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faConveyorBeltArm.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "conveyor-belt-arm";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e5f8";
    var svgPathData = "M240.9 0L320 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-52.9 0-5.7 34c.8 1 1.5 2 2.3 3l204.5 27.3c17.5 2.3 29.8 18.4 27.5 35.9c-1.4 10.2-7.4 18.7-15.7 23.5l0 20.3c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-17.9L255.2 164.5c-8.4 7.2-19.3 11.6-31.2 11.6c-26.5 0-48-21.5-48-48c0-17.1 8.9-32 22.3-40.6L202.2 64 160 64c-17.7 0-32-14.3-32-32s14.3-32 32-32l79 0c.6 0 1.2 0 1.9 0zM240 128a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zM416 280c0-13.3 10.7-24 24-24l48 0c13.3 0 24 10.7 24 24l0 40 32 0c53 0 96 43 96 96s-43 96-96 96L96 512c-53 0-96-43-96-96s43-96 96-96l0-40c0-13.3 10.7-24 24-24l48 0c13.3 0 24 10.7 24 24l0 40 64 0 0-40c0-13.3 10.7-24 24-24l48 0c13.3 0 24 10.7 24 24l0 40 64 0 0-40zM128 448a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm224-32a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm160 32a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faConveyorBeltArm = exports.definition;
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

// src/_entries/faConveyorBeltArm.js
var import_faConveyorBeltArm = __toESM(require_faConveyorBeltArm());
var export_faConveyorBeltArm = import_faConveyorBeltArm.definition;
export {
  export_faConveyorBeltArm as faConveyorBeltArm
};
