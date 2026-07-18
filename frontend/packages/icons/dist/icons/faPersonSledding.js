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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSledding.js
var require_faPersonSledding = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSledding.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-sledding";
    var width = 512;
    var height = 512;
    var aliases = ["sledding"];
    var unicode = "f7cb";
    var svgPathData = "M400 128a48 48 0 1 0 0-96 48 48 0 1 0 0 96zM35.1 226.7c-11.8-6.1-26.3-1.5-32.4 10.2s-1.5 26.3 10.2 32.4L396.4 468.4c27.3 14.2 59.7 14.4 87.2 .7l15.2-7.6c11.9-5.9 16.7-20.3 10.7-32.2s-20.3-16.7-32.2-10.7l-15.2 7.6c-13.7 6.9-29.9 6.7-43.6-.3l-41.7-21.6c4.5-5.5 7.2-12.5 7.2-20.2l0-80c0-26.5-21.5-48-48-48l-34.7 0L361 196.3c25.2-25.2 7.4-68.3-28.3-68.3L160 128c-17.7 0-32 14.3-32 32s14.3 32 32 32l84 0-59.2 40.3c-23.6 16.1-26 44.8-13.8 64.9L35.1 226.7zM320 320l0 54.6L214.7 320 320 320z";
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
    exports.faPersonSledding = exports.definition;
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

// src/_entries/faPersonSledding.js
var import_faPersonSledding = __toESM(require_faPersonSledding());
var export_faPersonSledding = import_faPersonSledding.definition;
export {
  export_faPersonSledding as faPersonSledding
};
