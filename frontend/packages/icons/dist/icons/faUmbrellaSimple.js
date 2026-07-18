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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUmbrellaSimple.js
var require_faUmbrellaSimple = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUmbrellaSimple.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "umbrella-simple";
    var width = 512;
    var height = 512;
    var aliases = ["umbrella-alt"];
    var unicode = "e2bc";
    var svgPathData = "M256 0c17.7 0 32 14.3 32 32l0 2C403.8 48.4 495.6 140.2 510 256.1c2.2 17.5-12.4 31.9-30 31.9L32 288C14.3 288-.2 273.6 2 256.1C16.4 140.2 108.2 48.4 224 34l0-2c0-17.7 14.3-32 32-32zm32 430.6c0 45-36.5 81.4-81.4 81.4c-30.8 0-59-17.4-72.8-45l-2.3-4.7c-7.9-15.8-1.5-35 14.3-42.9s35-1.5 42.9 14.3l2.3 4.7c3 5.9 9 9.6 15.6 9.6c9.6 0 17.4-7.8 17.4-17.4L224 320l64 0 0 110.6z";
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
    exports.faUmbrellaSimple = exports.definition;
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

// src/_entries/faUmbrellaSimple.js
var import_faUmbrellaSimple = __toESM(require_faUmbrellaSimple());
var export_faUmbrellaSimple = import_faUmbrellaSimple.definition;
export {
  export_faUmbrellaSimple as faUmbrellaSimple
};
