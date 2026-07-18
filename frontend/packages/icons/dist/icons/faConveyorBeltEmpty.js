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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faConveyorBeltEmpty.js
var require_faConveyorBeltEmpty = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faConveyorBeltEmpty.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "conveyor-belt-empty";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e150";
    var svgPathData = "M96 320c-53 0-96 43-96 96s43 96 96 96l448 0c53 0 96-43 96-96s-43-96-96-96L96 320zm32 64a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm160 32a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm224-32a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faConveyorBeltEmpty = exports.definition;
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

// src/_entries/faConveyorBeltEmpty.js
var import_faConveyorBeltEmpty = __toESM(require_faConveyorBeltEmpty());
var export_faConveyorBeltEmpty = import_faConveyorBeltEmpty.definition;
export {
  export_faConveyorBeltEmpty as faConveyorBeltEmpty
};
