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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSunCloud.js
var require_faSunCloud = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSunCloud.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sun-cloud";
    var width = 640;
    var height = 512;
    var aliases = [127780];
    var unicode = "f763";
    var svgPathData = "M361.5 1.2c5 2.1 8.6 6.6 9.6 11.9L391 121l107.9 19.8c5.3 1 9.8 4.6 11.9 9.6s1.5 10.7-1.6 15.2l-20.1 29.1C481 193 472.6 192 464 192c-32.9 0-62.4 14.2-82.9 36.7C368.6 171.1 317.3 128 256 128c-70.7 0-128 57.3-128 128s57.3 128 128 128c12.4 0 24.5-1.8 35.8-5.1c11.3 38.8 46.5 67.5 88.7 69l-9.4 51c-1 5.3-4.6 9.8-9.6 11.9s-10.7 1.5-15.2-1.6L256 446.9l-90.3 62.3c-4.5 3.1-10.2 3.7-15.2 1.6s-8.6-6.6-9.6-11.9L121 391 13.1 371.1c-5.3-1-9.8-4.6-11.9-9.6s-1.5-10.7 1.6-15.2L65.1 256 2.8 165.7c-3.1-4.5-3.7-10.2-1.6-15.2s6.6-8.6 11.9-9.6L121 121 140.9 13.1c1-5.3 4.6-9.8 9.6-11.9s10.7-1.5 15.2 1.6L256 65.1 346.3 2.8c4.5-3.1 10.2-3.7 15.2-1.6zM352 256c0 1.9-.1 3.7-.2 5.5c-35.6 12.6-61.5 45.6-63.7 85c-10.1 3.6-20.9 5.5-32.2 5.5c-53 0-96-43-96-96s43-96 96-96s96 43 96 96zm32 160c-35.3 0-64-28.7-64-64s28.7-64 64-64c.5 0 1.1 0 1.6 0c7.4-36.5 39.7-64 78.4-64c35 0 64.8 22.5 75.6 53.8c8.7-3.7 18.3-5.8 28.4-5.8c39.8 0 72 32.2 72 72s-32.2 72-72 72l-184 0z";
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
    exports.faSunCloud = exports.definition;
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

// src/_entries/faSunCloud.js
var import_faSunCloud = __toESM(require_faSunCloud());
var export_faSunCloud = import_faSunCloud.definition;
export {
  export_faSunCloud as faSunCloud
};
