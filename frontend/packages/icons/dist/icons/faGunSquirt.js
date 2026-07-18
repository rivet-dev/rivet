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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGunSquirt.js
var require_faGunSquirt = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGunSquirt.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "gun-squirt";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e19d";
    var svgPathData = "M128 56c0-13.3-10.7-24-24-24S80 42.7 80 56l0 9.3C34.6 72.9 0 112.4 0 160c0 47.3 34.3 86.7 79.4 94.6L33 440.2c-2.4 9.6-.2 19.7 5.8 27.5S54.1 480 64 480l104 0c14.7 0 27.5-10 31-24.2L219 376l53 0 0 16c0 13.3 10.7 24 24 24s24-10.7 24-24l0-40 0-40c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 16-41 0 18-72 199 0c41.8 0 77.4-26.7 90.5-64l13.5 0c13.3 0 24-10.7 24-24l0-16c0-13.3-10.7-24-24-24l-13.5 0C525.4 90.7 489.8 64 448 64L128 64l0-8zM96 128l352 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L96 192c-17.7 0-32-14.3-32-32s14.3-32 32-32z";
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
    exports.faGunSquirt = exports.definition;
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

// src/_entries/faGunSquirt.js
var import_faGunSquirt = __toESM(require_faGunSquirt());
var export_faGunSquirt = import_faGunSquirt.definition;
export {
  export_faGunSquirt as faGunSquirt
};
