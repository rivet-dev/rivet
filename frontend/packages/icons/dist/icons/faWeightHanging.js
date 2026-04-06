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

// src/node_modules/@fortawesome/free-solid-svg-icons/faWeightHanging.js
var require_faWeightHanging = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faWeightHanging.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "weight-hanging";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f5cd";
    var svgPathData = "M224 96a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm122.5 32c3.5-10 5.5-20.8 5.5-32 0-53-43-96-96-96s-96 43-96 96c0 11.2 1.9 22 5.5 32l-21.8 0c-28.3 0-53.2 18.5-61.3 45.6L2.4 440.2C.8 445.4 0 450.8 0 456.2 0 487 25 512 55.8 512l400.4 0c30.8 0 55.8-25 55.8-55.8 0-5.4-.8-10.8-2.4-16l-80-266.5c-8.1-27.1-33-45.6-61.3-45.6l-21.8 0z";
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
    exports.faWeightHanging = exports.definition;
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

// src/_entries/faWeightHanging.js
var import_faWeightHanging = __toESM(require_faWeightHanging());
var export_faWeightHanging = import_faWeightHanging.definition;
export {
  export_faWeightHanging as faWeightHanging
};
