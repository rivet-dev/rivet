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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTrianglePersonDigging.js
var require_faTrianglePersonDigging = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTrianglePersonDigging.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "triangle-person-digging";
    var width = 640;
    var height = 512;
    var aliases = ["construction"];
    var unicode = "f85d";
    var svgPathData = "M354.5 51.8C347.3 39.5 334.2 32 320 32s-27.3 7.5-34.5 19.8l-216 368c-7.3 12.4-7.3 27.7-.2 40.1S89.7 480 104 480l432 0c14.3 0 27.6-7.7 34.7-20.1s7-27.8-.2-40.1l-216-368zM216.9 304.7l15.6 9.8 64 40c4.7 2.9 7.5 8.1 7.5 13.6l0 32c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-23.1-40.9-25.5-24.5 55.2c-3.6 8.1-13 11.7-21.1 8.1s-11.7-13-8.1-21.1l32-72 7.5-16.8zm112-45.7l19.7 49.2L402 339l16.3-27.2c2.9-4.8 8.1-7.8 13.7-7.8s10.8 2.9 13.7 7.8l48 80c3 4.9 3 11.1 .2 16.1s-8.2 8.1-13.9 8.1l-96 0c-5.8 0-11.1-3.1-13.9-8.1s-2.8-11.2 .2-16.1l15.2-25.4L232 277.9c-3.8-2.2-6.6-5.9-7.6-10.2s-.2-8.8 2.3-12.5l4.2-6.4c10.3-15.5 27.7-24.8 46.3-24.8c22.7 0 43.2 13.8 51.6 34.9zM288 184a24 24 0 1 1 48 0 24 24 0 1 1 -48 0z";
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
    exports.faTrianglePersonDigging = exports.definition;
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

// src/_entries/faConstruction.js
var import_faTrianglePersonDigging = __toESM(require_faTrianglePersonDigging());
var export_faConstruction = import_faTrianglePersonDigging.definition;
export {
  export_faConstruction as faConstruction
};
