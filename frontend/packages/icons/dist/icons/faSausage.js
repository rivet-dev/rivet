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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSausage.js
var require_faSausage = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSausage.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sausage";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f820";
    var svgPathData = "M320 160c0-42 26.9-77.7 64.5-90.7L369.2 27.4c-.8-2.2-1.2-4.6-1.2-7C368 9.1 377.1 0 388.4 0l55.2 0C454.9 0 464 9.1 464 20.4c0 2.4-.4 4.7-1.2 7L447.5 69.3C485.1 82.3 512 118 512 160c0 194.4-157.6 352-352 352c-42 0-77.7-26.9-90.7-64.5L27.4 462.8c-2.2 .8-4.6 1.2-7 1.2C9.1 464 0 454.9 0 443.6l0-55.2C0 377.1 9.1 368 20.4 368c2.4 0 4.7 .4 7 1.2l41.9 15.2C82.3 346.9 118 320 160 320c88.4 0 160-71.6 160-160zm80 0c0-8.8-7.2-16-16-16s-16 7.2-16 16c0 114.9-93.1 208-208 208c-8.8 0-16 7.2-16 16s7.2 16 16 16c132.5 0 240-107.5 240-240z";
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
    exports.faSausage = exports.definition;
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

// src/_entries/faSausage.js
var import_faSausage = __toESM(require_faSausage());
var export_faSausage = import_faSausage.definition;
export {
  export_faSausage as faSausage
};
