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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCarGarage.js
var require_faCarGarage = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCarGarage.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "car-garage";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f5e2";
    var svgPathData = "M343.8 8.8c-14.8-8.4-32.9-8.4-47.6 0L24.1 164.2C8.8 173 3.5 192.5 12.2 207.9s28.3 20.7 43.7 11.9L320 68.9 584.1 219.8c15.3 8.8 34.9 3.4 43.7-11.9s3.4-34.9-11.9-43.7L343.8 8.8zM249.8 224l140.3 0c6.8 0 12.8 4.3 15.1 10.6L424.3 288l-208.6 0 19.1-53.4c2.3-6.4 8.3-10.6 15.1-10.6zm-75.3-10.9l-28.3 79.3C126.1 300.9 112 320.8 112 344l0 40 0 64 0 32c0 17.7 14.3 32 32 32l16 0c17.7 0 32-14.3 32-32l0-32 256 0 0 32c0 17.7 14.3 32 32 32l16 0c17.7 0 32-14.3 32-32l0-32 0-64 0-40c0-23.2-14.1-43.1-34.2-51.6l-28.3-79.3C454.1 181.3 424 160 390.2 160l-140.3 0c-33.8 0-64 21.3-75.3 53.1zM192 344a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm232 24a24 24 0 1 1 48 0 24 24 0 1 1 -48 0z";
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
    exports.faCarGarage = exports.definition;
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

// src/_entries/faCarGarage.js
var import_faCarGarage = __toESM(require_faCarGarage());
var export_faCarGarage = import_faCarGarage.definition;
export {
  export_faCarGarage as faCarGarage
};
