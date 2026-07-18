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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBanParking.js
var require_faBanParking = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBanParking.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ban-parking";
    var width = 512;
    var height = 512;
    var aliases = ["parking-circle-slash"];
    var unicode = "f616";
    var svgPathData = "M412.5 367.2L342.3 297c20.6-17.6 33.7-43.8 33.7-73c0-53-43-96-96-96l-72 0c-9.7 0-18.6 3.5-25.5 9.2L144.8 99.5C176.1 77.1 214.5 64 256 64c106 0 192 86 192 192c0 41.5-13.1 79.9-35.5 111.2zm-45.3 45.3C335.9 434.9 297.5 448 256 448C150 448 64 362 64 256c0-41.5 13.1-79.9 35.5-111.2L367.2 412.5zM296.6 251.4L237.3 192l42.7 0c17.7 0 32 14.3 32 32c0 11.6-6.2 21.7-15.4 27.4zM256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM232 325.3l-64-64 0 26.7 0 64c0 17.7 14.3 32 32 32s32-14.3 32-32l0-26.7z";
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
    exports.faBanParking = exports.definition;
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

// src/_entries/faBanParking.js
var import_faBanParking = __toESM(require_faBanParking());
var export_faBanParking = import_faBanParking.definition;
export {
  export_faBanParking as faBanParking
};
