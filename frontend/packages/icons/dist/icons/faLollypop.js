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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLollipop.js
var require_faLollipop = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLollipop.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lollipop";
    var width = 512;
    var height = 512;
    var aliases = [127853, "lollypop"];
    var unicode = "e424";
    var svgPathData = "M210.8 334.8C265.3 314.2 304 261.6 304 200l0-8c0-61.9-50.1-112-112-112S80 130.1 80 192c0 39.8 32.2 72 72 72s72-32.2 72-72l0-8c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 8c0 13.3-10.7 24-24 24s-24-10.7-24-24c0-35.3 28.7-64 64-64s64 28.7 64 64l0 8c0 53-43 96-96 96l-8 0C94.6 296 48 249.4 48 192c0-79.5 64.5-144 144-144s144 64.5 144 144c0 73.2-54.6 133.6-125.2 142.8zM0 192C0 298 86 384 192 384c41.5 0 79.9-13.1 111.2-35.5L457.4 502.6c12.5 12.5 32.8 12.5 45.3 0s12.5-32.8 0-45.3L348.5 303.2C370.9 271.9 384 233.5 384 192C384 86 298 0 192 0S0 86 0 192z";
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
    exports.faLollipop = exports.definition;
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

// src/_entries/faLollypop.js
var import_faLollipop = __toESM(require_faLollipop());
var export_faLollypop = import_faLollipop.definition;
export {
  export_faLollypop as faLollypop
};
