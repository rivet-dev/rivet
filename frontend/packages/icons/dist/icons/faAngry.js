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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFaceAngry.js
var require_faFaceAngry = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFaceAngry.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-angry";
    var width = 512;
    var height = 512;
    var aliases = [128544, "angry"];
    var unicode = "f556";
    var svgPathData = "M0 256a256 256 0 1 1 512 0 256 256 0 1 1 -512 0zM256 368c24.1 0 45.4 11.8 58.5 30 7.7 10.8 22.7 13.2 33.5 5.5s13.2-22.7 5.5-33.5c-21.7-30.2-57.3-50-97.5-50s-75.7 19.8-97.5 50c-7.7 10.8-5.3 25.8 5.5 33.5s25.8 5.3 33.5-5.5c13.1-18.2 34.4-30 58.5-30zm-80-96c17.7 0 32-14.3 32-32l0-.3 9.7 3.2c10.5 3.5 21.8-2.2 25.3-12.6s-2.2-21.8-12.6-25.3l-96-32c-10.5-3.5-21.8 2.2-25.3 12.6s2.2 21.8 12.6 25.3l28.9 9.6c-4.1 5.4-6.6 12.1-6.6 19.4 0 17.7 14.3 32 32 32zm192-32c0-7.3-2.4-14-6.6-19.4l28.9-9.6c10.5-3.5 16.1-14.8 12.6-25.3s-14.8-16.1-25.3-12.6l-96 32c-10.5 3.5-16.1 14.8-12.6 25.3s14.8 16.1 25.3 12.6l9.7-3.2 0 .3c0 17.7 14.3 32 32 32s32-14.3 32-32z";
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
    exports.faFaceAngry = exports.definition;
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

// src/_entries/faAngry.js
var import_faFaceAngry = __toESM(require_faFaceAngry());
var export_faAngry = import_faFaceAngry.definition;
export {
  export_faAngry as faAngry
};
