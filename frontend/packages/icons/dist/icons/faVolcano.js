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

// src/node_modules/@fortawesome/free-solid-svg-icons/faVolcano.js
var require_faVolcano = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faVolcano.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "volcano";
    var width = 512;
    var height = 512;
    var aliases = [127755];
    var unicode = "f770";
    var svgPathData = "M160 144c-35.3 0-64-28.7-64-64s28.7-64 64-64c15.7 0 30 5.6 41.2 15 11.2-18.6 31.6-31 54.8-31s43.6 12.4 54.8 31c11.1-9.4 25.5-15 41.2-15 35.3 0 64 28.7 64 64s-28.7 64-64 64c-14.7 0-28.3-5-39.1-13.3l-32 48C275.3 187 266 192 256 192s-19.3-5-24.9-13.3l-32-48C188.3 139 174.7 144 160 144zM144 352l48.4-24.2c10.2-5.1 21.6-7.8 33-7.8 19.6 0 38.4 7.8 52.2 21.6l32.5 32.5c6.3 6.3 14.9 9.9 23.8 9.9 11.3 0 21.8-5.6 28-15l9.7-14.6-59-66.3c-9.1-10.2-22.2-16.1-35.9-16.1l-41.8 0c-13.7 0-26.8 5.9-35.9 16.1l-59.9 67.4 4.7-3.5zm19.4-95.8c18.2-20.5 44.3-32.2 71.8-32.2l41.8 0c27.4 0 53.5 11.7 71.8 32.2l150.2 169c8.5 9.5 13.2 21.9 13.2 34.7 0 28.8-23.4 52.2-52.2 52.2L52.2 512C23.4 512 0 488.6 0 459.8 0 447.1 4.7 434.7 13.2 425.2l150.2-169z";
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
    exports.faVolcano = exports.definition;
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

// src/_entries/faVolcano.js
var import_faVolcano = __toESM(require_faVolcano());
var export_faVolcano = import_faVolcano.definition;
export {
  export_faVolcano as faVolcano
};
