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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPhoneOffice.js
var require_faPhoneOffice = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPhoneOffice.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "phone-office";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f67d";
    var svgPathData = "M128 0C110.3 0 96 14.3 96 32l0 352c0 17.7 14.3 32 32 32l64 0c17.7 0 32-14.3 32-32l0-352c0-17.7-14.3-32-32-32L128 0zM64 32C28.7 32 0 60.7 0 96L0 448c0 35.3 28.7 64 64 64l448 0c35.3 0 64-28.7 64-64l0-352c0-35.3-28.7-64-64-64L256 32l0 352c0 35.3-28.7 64-64 64l-64 0c-35.3 0-64-28.7-64-64L64 32zm256 96c0-17.7 14.3-32 32-32l128 0c17.7 0 32 14.3 32 32l0 32c0 17.7-14.3 32-32 32l-128 0c-17.7 0-32-14.3-32-32l0-32zm32 192a32 32 0 1 1 0-64 32 32 0 1 1 0 64zm160-32a32 32 0 1 1 -64 0 32 32 0 1 1 64 0zM480 448a32 32 0 1 1 0-64 32 32 0 1 1 0 64zm-96-32a32 32 0 1 1 -64 0 32 32 0 1 1 64 0z";
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
    exports.faPhoneOffice = exports.definition;
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

// src/_entries/faPhoneOffice.js
var import_faPhoneOffice = __toESM(require_faPhoneOffice());
var export_faPhoneOffice = import_faPhoneOffice.definition;
export {
  export_faPhoneOffice as faPhoneOffice
};
