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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFaceGrinBeam.js
var require_faFaceGrinBeam = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFaceGrinBeam.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-grin-beam";
    var width = 512;
    var height = 512;
    var aliases = [128516, "grin-beam"];
    var unicode = "f582";
    var svgPathData = "M256 512a256 256 0 1 0 0-512 256 256 0 1 0 0 512zM386.7 308.9c11.9-3.7 23.9 6.3 19.6 18.1-22.4 61.3-81.3 105.1-150.3 105.1S128.1 388.2 105.7 326.9c-4.3-11.8 7.7-21.8 19.6-18.1 39.2 12.2 83.7 19.1 130.7 19.1s91.5-6.9 130.7-19.1zM176 180c-15.5 0-28 12.5-28 28l0 8c0 11-9 20-20 20s-20-9-20-20l0-8c0-37.6 30.4-68 68-68s68 30.4 68 68l0 8c0 11-9 20-20 20s-20-9-20-20l0-8c0-15.5-12.5-28-28-28zm132 28l0 8c0 11-9 20-20 20s-20-9-20-20l0-8c0-37.6 30.4-68 68-68s68 30.4 68 68l0 8c0 11-9 20-20 20s-20-9-20-20l0-8c0-15.5-12.5-28-28-28s-28 12.5-28 28z";
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
    exports.faFaceGrinBeam = exports.definition;
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

// src/_entries/faGrinBeam.js
var import_faFaceGrinBeam = __toESM(require_faFaceGrinBeam());
var export_faGrinBeam = import_faFaceGrinBeam.definition;
export {
  export_faGrinBeam as faGrinBeam
};
