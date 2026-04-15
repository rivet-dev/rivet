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

// src/node_modules/@fortawesome/free-solid-svg-icons/faStaffSnake.js
var require_faStaffSnake = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faStaffSnake.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "staff-snake";
    var width = 448;
    var height = 512;
    var aliases = ["rod-asclepius", "rod-snake", "staff-aesculapius"];
    var unicode = "e579";
    var svgPathData = "M192 32l0-32c0-17.7 14.3-32 32-32s32 14.3 32 32l0 32 64 0c53 0 96 43 96 96s-43 96-96 96l-16 0 0-64 16 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-64 0 0 192 32 0c53 0 96 43 96 96 0 47.6-34.6 87.1-80 94.7l0-67c9.6-5.5 16-15.9 16-27.7 0-17.7-14.3-32-32-32l-32 0 0 160c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32-32 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l32 0 0-64-32 0c-53 0-96-43-96-96 0-47.6 34.6-87.1 80-94.7l0 67c-9.6 5.5-16 15.9-16 27.7 0 17.7 14.3 32 32 32l32 0 0-192-72.6 0c-11.1 19.1-31.7 32-55.4 32l-16 0C21.5 128 0 106.5 0 80S21.5 32 48 32l144 0z";
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
    exports.faStaffSnake = exports.definition;
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

// src/_entries/faStaffAesculapius.js
var import_faStaffSnake = __toESM(require_faStaffSnake());
var export_faStaffAesculapius = import_faStaffSnake.definition;
export {
  export_faStaffAesculapius as faStaffAesculapius
};
